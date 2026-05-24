use chrono::Utc;
use secrecy::SecretBox;
use tauri::{AppHandle, Emitter, Manager};
use zeroize::Zeroizing;

use crate::crypto::{derive_keys, encrypt_totp_secret};
use crate::db::{self, meta, users};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RegisterDraft};
use crate::util::trace::AuthTimer;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProgress {
    pub step: String,
    pub status: String,
    pub message: Option<String>,
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

fn emit_progress(app: &AppHandle, step: &str, status: &str, message: Option<String>) {
    let _ = app.emit(
        "register-progress",
        RegisterProgress {
            step: step.to_string(),
            status: status.to_string(),
            message,
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
            emit_progress(&app_handle, "error", "error", Some(e.to_string()));
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
    for step in STEPS {
        emit_progress(app, step, "running", None);

        match *step {
            "validate_draft" => validate_draft(&draft)?,
            "create_data_dir" => {
                meta::ensure_argus_dir()?;
            }
            "open_database" | "run_migrations" | "derive_keys" | "persist_user" | "open_session" => {
                if *step == "open_session" {
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
            }
            _ => {}
        }

        emit_progress(app, step, "done", None);
    }

    Ok(())
}

fn validate_draft(draft: &RegisterDraft) -> AppResult<()> {
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
) -> AppResult<()> {
    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if inner.is_signed_in() {
            return Ok(());
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

    Ok(())
}
