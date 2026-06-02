use chrono::Utc;
use secrecy::SecretBox;
use tauri::{AppHandle, Emitter, Manager};
use zeroize::Zeroizing;

use crate::crypto::{derive_keys, encrypt_totp_secret};
use crate::infra::db::{self, meta, users};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RegisterDraft};
use crate::util::trace::AuthTimer;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProgress {
    pub step: String,
    pub status: String,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_code: Option<String>,
}

const STEPS: &[&str] = &[
    "validate_draft",
    "create_data_dir",
    "open_database",
    "run_migrations",
    "derive_keys",
    "persist_user",
    "open_session",
    "complete",
];

fn emit_progress(
    app: &AppHandle,
    step: &str,
    status: &str,
    message: Option<String>,
    recovery_code: Option<String>,
) {
    let _ = app.emit(
        "register-progress",
        RegisterProgress {
            step: step.to_string(),
            status: status.to_string(),
            message,
            recovery_code,
        },
    );
}

pub fn run_finalize(app: AppHandle, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let draft = {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

        if inner.is_signed_in() {
            return Ok(());
        }

        if inner.register_finalize_running {
            return Ok(());
        }

        let draft = inner.register_draft.clone().ok_or_else(|| {
            AppError::message("NO_REGISTER_DRAFT", "registration session expired; start again")
        })?;

        inner.register_finalize_running = true;
        draft
    };

    let timer = AuthTimer::start("register_finalize");
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let result = finalize_inner(&app_handle, &state, draft);
        if let Err(e) = result {
            emit_progress(&app_handle, "error", "error", Some(e.to_string()), None);
        }

        if let Ok(mut inner) = state.0.lock() {
            inner.register_finalize_running = false;
        }

        timer.done();
    });

    Ok(())
}

fn finalize_inner(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    draft: RegisterDraft,
) -> AppResult<()> {
    let mut registration_recovery_code: Option<String> = None;

    for step in STEPS {
        emit_progress(app, step, "running", None, None);

        match *step {
            "validate_draft" => validate_draft(&draft)?,
            "create_data_dir" => {
                meta::ensure_argus_dir()?;
            }
            "open_database" | "run_migrations" | "derive_keys" | "persist_user" | "open_session" => {
                if *step == "open_session" {
                    registration_recovery_code =
                        establish_session(app, state, &draft)?;
                }
            }
            "complete" => {
                let profile = {
                    let inner = state.0.lock().map_err(|_| {
                        AppError::message("LOCK_ERROR", "state poisoned")
                    })?;
                    let pool = inner.db.as_ref().ok_or_else(|| {
                        AppError::message("DB_ERROR", "session not established")
                    })?;
                    let conn = pool.lock().map_err(|_| {
                        AppError::message("LOCK_ERROR", "db poisoned")
                    })?;
                    users::get_profile(&conn)?
                };
                let _ = app.emit("signed-in", profile);
                let scopes = {
                    let inner = state.0.lock().map_err(|_| {
                        AppError::message("LOCK_ERROR", "state poisoned")
                    })?;
                    inner.scope_status()
                };
                let _ = app.emit("scope-changed", scopes);
                crate::ipc::start_for_app(app);
                crate::proxy::start_for_app(app);
            }
            _ => {}
        }

        emit_progress(
            app,
            step,
            "done",
            None,
            if *step == "complete" {
                registration_recovery_code.clone()
            } else {
                None
            },
        );
    }

    Ok(())
}

fn validate_draft(draft: &RegisterDraft) -> AppResult<()> {
    if draft.first_name.is_empty() || draft.last_name.is_empty() {
        return Err(AppError::message("VALIDATION_ERROR", "missing name fields"));
    }
    if draft.email.is_empty() || draft.username.is_empty() {
        return Err(AppError::message("VALIDATION_ERROR", "missing account fields"));
    }
    if draft.second_factor_type == "totp" && draft.totp_secret_plain.is_none() {
        return Err(AppError::message("VALIDATION_ERROR", "TOTP not configured"));
    }
    if draft.second_factor_type == "biometric" && !draft.biometric_enrolled {
        return Err(AppError::message("VALIDATION_ERROR", "biometric not enrolled"));
    }
    Ok(())
}

fn establish_session(
    _app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    draft: &RegisterDraft,
) -> AppResult<Option<String>> {
    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if inner.is_signed_in() {
            return Ok(None);
        }
    }

    let password = Zeroizing::new(draft.password.clone());
    let keys = derive_keys(&password, &draft.password_hash)?;

    let totp_enc = if draft.second_factor_type == "totp" {
        let secret = draft.totp_secret_plain.as_deref().ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "missing TOTP secret")
        })?;
        Some(encrypt_totp_secret(&keys.value_key, secret)?)
    } else {
        None
    };

    let pool = db::open_db(&keys.db_key)?;
    {
        let conn = pool.lock().map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;

        users::insert_user(
            &conn,
            &draft.email,
            &draft.username,
            &draft.first_name,
            &draft.last_name,
            &draft.password_hash,
            totp_enc.as_deref(),
            &draft.second_factor_type,
            draft.second_factor_type == "totp",
            draft.biometric_enrolled,
        )?;

        users::update_last_signed_in(&conn)?;
    }

    meta::write_password_hash(&draft.password_hash)?;
    meta::write_account_meta(&draft.second_factor_type)?;
    if let Some(ref enc) = totp_enc {
        meta::write_totp_secret_enc(enc)?;
    }

    let (recovery_code, _) =
        crate::api::recovery::persist_registration_recovery(&keys.db_key, &keys.value_key)?;

    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    inner.db = Some(pool);
    inner.db_key = Some(SecretBox::new(Box::new(keys.db_key)));
    inner.value_key = Some(SecretBox::new(Box::new(keys.value_key)));
    inner.signed_in_at = Some(Utc::now());
    inner.password_hash_cache = Some(draft.password_hash.clone());
    inner.register_draft = None;
    inner.pending_sign_in = None;
    inner.app_locked = false;
    inner.registration_recovery_code = Some(recovery_code.clone());

    Ok(Some(recovery_code))
}
