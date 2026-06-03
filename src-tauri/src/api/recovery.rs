use chrono::{Duration, Utc};
use secrecy::{ExposeSecret, SecretBox};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::crypto::{
    derive_keys, derive_recovery_key, encrypt_totp_secret, hash_password, hash_recovery_code,
    is_valid_recovery_code, normalize_recovery_code, unwrap_session_keys,
    verify_recovery_code as crypto_verify_recovery_code, verify_totp_code, wrap_session_keys,
};
use crate::infra::db::{self, meta, rekey, users};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, RecoverySession};
use crate::util::{biometry, limit};

const RECOVERY_SESSION_MINUTES: i64 = 15;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryVerifyResponse {
    pub signed_in: bool,
    pub app_locked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCodeRequest {
    pub recovery_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResetSecondFactorRequest {
    pub second_factor_type: String,
    pub totp_secret: Option<String>,
    pub totp_code: Option<String>,
}

#[tauri::command]
pub fn verify_account_recovery(
    state: State<'_, AppState>,
    req: RecoveryCodeRequest,
) -> Result<RecoveryVerifyResponse, String> {
    run_verify_recovery_code(&state, &req.recovery_code).map_err(|e| String::from(e))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPasswordResetResponse {
    pub username: String,
    pub requires_second_factor: bool,
}

#[tauri::command]
pub fn recovery_reset_password(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RecoveryResetPasswordRequest,
) -> Result<RecoveryPasswordResetResponse, String> {
    run_recovery_reset_password(&app, &state, &req.new_password).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn recovery_reset_second_factor(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RecoveryResetSecondFactorRequest,
) -> Result<String, String> {
    run_recovery_reset_second_factor(&app, &state, req).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn take_registration_recovery_code(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    Ok(inner.registration_recovery_code.take())
}

fn run_verify_recovery_code(
    state: &State<'_, AppState>,
    raw_code: &str,
) -> AppResult<RecoveryVerifyResponse> {
    if !meta::read_has_account()? {
        return Err(AppError::message("NO_ACCOUNT", "no account registered"));
    }

    if !is_valid_recovery_code(raw_code) {
        return Err(AppError::message("VALIDATION_ERROR", "invalid recovery code"));
    }

    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::check_lockout(&inner)?;
    }

    let recovery_hash = meta::read_recovery_hash().map_err(|_| {
        AppError::message(
            "RECOVERY_UNAVAILABLE",
            "recovery code not configured for this account",
        )
    })?;

    if !crypto_verify_recovery_code(raw_code, &recovery_hash)? {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::record_failure(&mut inner);
        return Err(AppError::message("AUTH_FAILED", "invalid recovery code"));
    }

    let signed_in;
    let app_locked;
    {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        signed_in = inner.is_signed_in();
        app_locked = inner.app_locked;
    }

    let mut recovery_db = None;
    let mut recovery_db_key = None;
    let mut recovery_value_key = None;

    if !signed_in {
        let escrow = meta::read_recovery_escrow()?;
        let code = normalize_recovery_code(raw_code);
        let recovery_key = derive_recovery_key(&code, &recovery_hash)?;
        let (db_key, value_key) = unwrap_session_keys(&recovery_key, &escrow)?;
        let pool = db::open_db(&db_key)?;
        recovery_db = Some(pool);
        recovery_db_key = Some(SecretBox::new(Box::new(db_key)));
        recovery_value_key = Some(SecretBox::new(Box::new(value_key)));
    }

    {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        limit::clear_failures(&mut inner);
        inner.recovery_session = Some(RecoverySession {
            verified_at: Utc::now(),
            recovery_code: normalize_recovery_code(raw_code),
            recovery_db,
            recovery_db_key,
            recovery_value_key,
        });
    }

    Ok(RecoveryVerifyResponse {
        signed_in,
        app_locked,
    })
}

fn ensure_recovery_session(state: &State<'_, AppState>) -> AppResult<()> {
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    let session = inner
        .recovery_session
        .as_ref()
        .ok_or_else(|| AppError::message("RECOVERY_REQUIRED", "verify recovery code first"))?;
    let age = Utc::now() - session.verified_at;
    if age > Duration::minutes(RECOVERY_SESSION_MINUTES) {
        return Err(AppError::message(
            "RECOVERY_EXPIRED",
            "recovery session expired; verify code again",
        ));
    }
    Ok(())
}

fn with_recovery_conn<R>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&rusqlite::Connection) -> AppResult<R>,
) -> AppResult<R> {
    ensure_recovery_session(state)?;
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    if inner.is_signed_in() {
        let pool = inner
            .db
            .as_ref()
            .ok_or_else(|| AppError::message("DB_ERROR", "session not established"))?;
        let conn = pool
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
        return f(&conn);
    }

    let session = inner
        .recovery_session
        .as_ref()
        .ok_or_else(|| AppError::message("RECOVERY_REQUIRED", "verify recovery code first"))?;
    let pool = session
        .recovery_db
        .as_ref()
        .ok_or_else(|| AppError::message("DB_ERROR", "recovery database not open"))?;
    let conn = pool
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
    f(&conn)
}

fn session_keys_for_recovery(
    state: &State<'_, AppState>,
) -> AppResult<([u8; 32], [u8; 32])> {
    ensure_recovery_session(state)?;
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

    if inner.is_signed_in() {
        let db_key = inner
            .db_key
            .as_ref()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        let value_key = inner
            .value_key
            .as_ref()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        return Ok((
            *db_key.expose_secret(),
            *value_key.expose_secret(),
        ));
    }

    let session = inner
        .recovery_session
        .as_ref()
        .ok_or_else(|| AppError::message("RECOVERY_REQUIRED", "verify recovery code first"))?;
    let db_key = session
        .recovery_db_key
        .as_ref()
        .ok_or_else(|| AppError::message("DB_ERROR", "recovery keys missing"))?;
    let value_key = session
        .recovery_value_key
        .as_ref()
        .ok_or_else(|| AppError::message("DB_ERROR", "recovery keys missing"))?;
    Ok((
        *db_key.expose_secret(),
        *value_key.expose_secret(),
    ))
}

fn run_recovery_reset_password(
    app: &AppHandle,
    state: &State<'_, AppState>,
    new_password: &str,
) -> AppResult<RecoveryPasswordResetResponse> {
    if new_password.len() < 10 {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "password must be at least 10 characters",
        ));
    }

    let (old_db_key, old_value_key) = session_keys_for_recovery(state)?;
    let _ = old_db_key;
    let _old_password_hash = meta::read_password_hash()?;
    let new_password_hash = hash_password(new_password)?;
    let new_keys = derive_keys(new_password, &new_password_hash)?;

    let profile = with_recovery_conn(state, |conn| {
        rekey::reencrypt_all_values(conn, &old_value_key, &new_keys.value_key)?;
        rekey::sync_totp_keychain(conn)?;
        users::update_password_hash(conn, &new_password_hash)?;
        rekey::rekey_database(conn, &new_keys.db_key)?;
        users::get_profile(conn)
    })?;

    update_recovery_escrow(state, &new_keys.db_key, &new_keys.value_key)?;

    let was_signed_in = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner.is_signed_in()
    };

    {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

        if was_signed_in {
            crate::ipc::stop_for_app(app);
            crate::proxy::stop_for_app(app);
        }

        inner.clear_session();
        inner.recovery_session = None;
        inner.pending_sign_in = None;
    }

    meta::write_password_hash(&new_password_hash)?;

    let _ = app.emit("signed-out", ());

    Ok(RecoveryPasswordResetResponse {
        username: profile.username,
        requires_second_factor: true,
    })
}

fn inner_recovery_code_from_session(state: &State<'_, AppState>) -> AppResult<String> {
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    inner
        .recovery_session
        .as_ref()
        .map(|s| s.recovery_code.clone())
        .ok_or_else(|| AppError::message("RECOVERY_REQUIRED", "recovery code missing from session"))
}

fn update_recovery_escrow(
    state: &State<'_, AppState>,
    db_key: &[u8; 32],
    value_key: &[u8; 32],
) -> AppResult<()> {
    let code = inner_recovery_code_from_session(state)?;
    let recovery_hash = meta::read_recovery_hash()?;
    let recovery_key = derive_recovery_key(&code, &recovery_hash)?;
    let escrow = wrap_session_keys(&recovery_key, db_key, value_key)?;
    meta::write_recovery_escrow(&recovery_hash, &escrow)
}

fn run_recovery_reset_second_factor(
    app: &AppHandle,
    state: &State<'_, AppState>,
    req: RecoveryResetSecondFactorRequest,
) -> AppResult<String> {
    ensure_recovery_session(state)?;

    let second_factor = req.second_factor_type.to_lowercase();
    if second_factor != "totp" && second_factor != "biometric" {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "second factor must be totp or biometric",
        ));
    }

    let (_, value_key) = session_keys_for_recovery(state)?;

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

    let totp_enc = if second_factor == "totp" {
        let secret = req.totp_secret.as_deref().ok_or_else(|| {
            AppError::message("VALIDATION_ERROR", "TOTP secret required")
        })?;
        Some(encrypt_totp_secret(&value_key, secret)?)
    } else {
        None
    };

    with_recovery_conn(state, |conn| {
        users::replace_second_factor(conn, &second_factor, totp_enc.as_deref(), biometric_enrolled)
    })?;

    meta::update_second_factor_type(&second_factor)?;
    if second_factor == "totp" {
        if let Some(ref enc) = totp_enc {
            meta::write_totp_secret_enc(enc)?;
        }
    } else {
        meta::clear_totp_secret_enc()?;
    }

    let was_signed_in = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner.is_signed_in()
    };

    {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if was_signed_in {
            crate::ipc::stop_for_app(app);
            crate::proxy::stop_for_app(app);
        }
        inner.clear_session();
        inner.recovery_session = None;
    }

    let _ = app.emit("signed-out", ());

    Ok(second_factor)
}

pub fn persist_registration_recovery(
    db_key: &[u8; 32],
    value_key: &[u8; 32],
) -> AppResult<(String, String)> {
    let code = crate::crypto::generate_recovery_code();
    let recovery_hash = hash_recovery_code(&code)?;
    let recovery_key = derive_recovery_key(&code, &recovery_hash)?;
    let escrow = wrap_session_keys(&recovery_key, db_key, value_key)?;
    meta::write_recovery_escrow(&recovery_hash, &escrow)?;
    Ok((code, recovery_hash))
}
