use chrono::Utc;
use secrecy::SecretBox;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use zeroize::Zeroizing;

use crate::crypto::{
    decrypt_totp_secret, derive_keys, generate_totp_secret, hash_password, verify_password,
    verify_totp_code, TotpSetup,
};
use crate::db::{self, meta, users};
use crate::db::users::UserProfile;
use crate::error::{AppError, AppResult, ErrorPayload};
use crate::register::finalize;
use crate::state::{AppState, PendingSignIn, RegisterDraft};
use crate::state::ScopeStatus;
use crate::util::{biometry, limit, second_factor, session};

#[tauri::command]
pub fn has_account() -> Result<bool, String> {
    meta::read_has_account().map_err(|e| String::from(e))
}

#[tauri::command]
pub fn prepare_totp_setup(account_label: String) -> Result<TotpSetup, String> {
    generate_totp_secret(&account_label).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn verify_biometric(app: AppHandle) -> Result<(), String> {
    biometry::verify_user("Verify your identity for Argus", &app).map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterValidateRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub second_factor_type: String,
    pub totp_secret: Option<String>,
    pub totp_code: Option<String>,
}

#[tauri::command]
pub fn register_validate(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RegisterValidateRequest,
) -> Result<(), String> {
    run_register_validate(&app, &state, req).map_err(|e| String::from(e))
}

fn run_register_validate(
    app: &AppHandle,
    state: &State<'_, AppState>,
    req: RegisterValidateRequest,
) -> AppResult<()> {
    validate_account_fields(&req)?;

    if meta::read_has_account()? {
        return Err(AppError::message("ACCOUNT_EXISTS", "account already registered"));
    }

    let second_factor = req.second_factor_type.to_lowercase();
    if second_factor != "totp" && second_factor != "biometric" {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "second factor must be totp or biometric",
        ));
    }

    let biometric_enrolled = if second_factor == "totp" {
        let secret = req.totp_secret.as_deref().ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "TOTP secret required")
        })?;
        let code = req.totp_code.as_deref().ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "TOTP code required")
        })?;
        if !verify_totp_code(secret, code)? {
            return Err(AppError::message("AUTH_FAILED", "invalid TOTP code"));
        }
        false
    } else {
        biometry::verify_user("Register biometric unlock for Argus", app)?;
        true
    };

    let password_hash = hash_password(&req.password)?;

    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    inner.register_draft = Some(RegisterDraft {
        email: req.email.trim().to_lowercase(),
        username: req.username.trim().to_string(),
        password: req.password,
        password_hash,
        second_factor_type: second_factor,
        totp_secret_plain: req.totp_secret,
        biometric_enrolled,
    });

    Ok(())
}

#[tauri::command]
pub fn register_finalize(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    finalize::run_finalize(app, state).map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInRequest {
    pub identifier: String,
    pub password: String,
    pub totp_code: Option<String>,
    /// User intends biometric step (verified in Rust, not trusted as proof).
    pub use_biometric: Option<bool>,
}

#[tauri::command]
pub fn sign_in(
    app: AppHandle,
    state: State<'_, AppState>,
    req: SignInRequest,
) -> Result<UserProfile, String> {
    match run_sign_in(&app, &state, req) {
        Ok(profile) => Ok(profile),
        Err(AppError::Message { code, message }) if code == "SECOND_FACTOR_REQUIRED" => {
            let second_factor_type = meta::read_second_factor_type().ok();
            Err(serde_json::to_string(&ErrorPayload {
                code: code.to_string(),
                message,
                second_factor_type,
            })
            .unwrap())
        }
        Err(e) => Err(String::from(e)),
    }
}

fn run_sign_in(
    app: &AppHandle,
    state: &State<'_, AppState>,
    req: SignInRequest,
) -> AppResult<UserProfile> {
    let has_2fa = req.totp_code.is_some() || req.use_biometric.unwrap_or(false);

    if !has_2fa {
        return password_step(state, &req.identifier, &req.password);
    }

    complete_sign_in(app, state, req)
}

fn password_step(
    state: &State<'_, AppState>,
    identifier: &str,
    password: &str,
) -> AppResult<UserProfile> {
    if !meta::read_has_account()? {
        return Err(AppError::message("NO_ACCOUNT", "no account registered"));
    }

    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::check_lockout(&inner)?;
    }

    let password_hash = meta::read_password_hash()?;
    if !verify_password(password, &password_hash)? {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::record_failure(&mut inner);
        return Err(AppError::message("AUTH_FAILED", "invalid credentials"));
    }

    let second_factor_type = meta::read_second_factor_type()?;

    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    inner.pending_sign_in = Some(PendingSignIn {
        identifier: identifier.trim().to_string(),
        password: password.to_string(),
        second_factor_type: second_factor_type.clone(),
    });

    Err(AppError::message(
        "SECOND_FACTOR_REQUIRED",
        format!("Second factor required ({second_factor_type})"),
    ))
}

fn complete_sign_in(
    app: &AppHandle,
    state: &State<'_, AppState>,
    req: SignInRequest,
) -> AppResult<UserProfile> {
    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::check_lockout(&inner)?;
    }

    let pending = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner.pending_sign_in.clone()
    };

    let pending = pending.ok_or_else(|| {
        AppError::message("AUTH_FAILED", "complete password step first")
    })?;

    if pending.identifier != req.identifier.trim() || pending.password != req.password {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::record_failure(&mut inner);
        return Err(AppError::message("AUTH_FAILED", "invalid credentials"));
    }

    let password_hash = meta::read_password_hash()?;
    let second_factor = pending.second_factor_type.to_lowercase();

    let result = if second_factor == "totp" {
        let code = req.totp_code.as_deref().ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "TOTP code required")
        })?;
        let secret_enc = meta::read_totp_secret_enc()?;
        let keys = derive_keys(&pending.password, &password_hash)?;
        let secret_b32 = decrypt_totp_secret(&keys.value_key, &secret_enc)?;
        if !verify_totp_code(&secret_b32, code)? {
            Err(AppError::message("AUTH_FAILED", "invalid TOTP code"))
        } else {
            establish_sign_in_session(app, state, &pending.password, &password_hash)
        }
    } else if second_factor == "biometric" {
        if !req.use_biometric.unwrap_or(false) {
            Err(AppError::message(
                "VALIDATION_ERROR",
                "biometric step required",
            ))
        } else {
            biometry::verify_user("Sign in to Argus", app)?;
            establish_sign_in_session(app, state, &pending.password, &password_hash)
        }
    } else {
        Err(AppError::message("AUTH_FAILED", "unknown second factor"))
    };

    if result.is_err() {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::record_failure(&mut inner);
    }

    result
}

fn establish_sign_in_session(
    app: &AppHandle,
    state: &State<'_, AppState>,
    password: &str,
    password_hash: &str,
) -> AppResult<UserProfile> {
    let pw = Zeroizing::new(password.to_string());
    let keys = derive_keys(&pw, password_hash)?;
    let pool = db::open_db(&keys.db_key)?;

    let profile = {
        let conn = pool
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
        users::update_last_signed_in(&conn)?;
        users::get_profile(&conn)?
    };

    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    inner.db = Some(pool);
    inner.db_key = Some(SecretBox::new(Box::new(keys.db_key)));
    inner.value_key = Some(SecretBox::new(Box::new(keys.value_key)));
    inner.signed_in_at = Some(Utc::now());
    inner.password_hash_cache = Some(password_hash.to_string());
    inner.pending_sign_in = None;
    inner.register_draft = None;
    inner.app_locked = false;
    limit::clear_failures(&mut inner);

    inner.touch_activity();
    let scopes = inner.scope_status();
    drop(inner);

    let _ = app.emit("signed-in", profile.clone());
    let _ = app.emit("scope-changed", scopes);

    crate::ipc::start_for_app(app);

    Ok(profile)
}

#[tauri::command]
pub fn sign_out(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    inner.clear_session();
    inner.register_draft = None;
    drop(inner);
    crate::ipc::stop_for_app(&app);
    let _ = app.emit("signed-out", ());
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockAppRequest {
    pub totp_code: Option<String>,
    pub use_biometric: Option<bool>,
}

#[tauri::command]
pub fn unlock_app(
    app: AppHandle,
    state: State<'_, AppState>,
    req: UnlockAppRequest,
) -> Result<ScopeStatus, String> {
    run_unlock_app(&app, &state, req).map_err(|e| String::from(e))
}

fn run_unlock_app(
    app: &AppHandle,
    state: &State<'_, AppState>,
    req: UnlockAppRequest,
) -> AppResult<ScopeStatus> {
    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if !inner.is_signed_in() {
            return Err(AppError::message("NOT_SIGNED_IN", "not signed in"));
        }
        if !inner.app_locked {
            return Ok(inner.scope_status());
        }
    }

    second_factor::verify_second_factor(
        app,
        state,
        second_factor::SecondFactorProof {
            totp_code: req.totp_code.as_deref(),
            use_biometric: req.use_biometric.unwrap_or(false),
        },
    )?;

    let scopes = {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner.app_locked = false;
        inner.touch_activity();
        limit::clear_failures(&mut inner);
        inner.scope_status()
    };

    let _ = app.emit("scope-changed", scopes.clone());
    Ok(scopes)
}

#[tauri::command]
pub fn lock_app(app: AppHandle, state: State<'_, AppState>) -> Result<ScopeStatus, String> {
    session::soft_lock_app(&app, &state).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn get_scope_status(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::state::ScopeStatus, String> {
    let signed_in = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner.is_signed_in()
    };
    if !signed_in {
        return Ok(crate::state::ScopeStatus {
            app: false,
            vault: false,
            buckets: false,
            vault_expires_at: None,
            buckets_expires_at: None,
        });
    }
    // Poll-only: check idle without resetting the activity timer.
    session::poll_idle_app_lock(&app, &state).map_err(|e| String::from(e))?;
    session::scope_status_after_sync(&app, &state).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn get_profile(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UserProfile, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    let pool = inner
        .db
        .as_ref()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let conn = pool
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
    users::get_profile(&conn).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn get_second_factor_type() -> Result<String, String> {
    meta::read_second_factor_type().map_err(|e| String::from(e))
}

fn validate_account_fields(req: &RegisterValidateRequest) -> AppResult<()> {
    let email = req.email.trim();
    let username = req.username.trim();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::message("VALIDATION_ERROR", "invalid email"));
    }
    if username.len() < 2 {
        return Err(AppError::message("VALIDATION_ERROR", "username too short"));
    }
    if req.password.len() < 10 {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "password must be at least 10 characters",
        ));
    }
    Ok(())
}
