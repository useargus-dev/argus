use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::crypto::{encrypt_totp_secret, verify_totp_code};
use crate::db::{meta, users};
use crate::db::users::UserProfile;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::util::{biometry, session};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondFactorStatus {
    pub active_second_factor: String,
    pub totp_enrolled: bool,
    pub biometric_enrolled: bool,
}

#[tauri::command]
pub fn get_second_factor_status(state: State<'_, AppState>) -> Result<SecondFactorStatus, String> {
    session::with_db(&state, |conn, _inner| {
        let user = users::get_user(conn)?;
        Ok(SecondFactorStatus {
            active_second_factor: user.second_factor_type,
            totp_enrolled: user.totp_enabled,
            biometric_enrolled: user.biometric_enrolled,
        })
    })
    .map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub email: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[tauri::command]
pub fn update_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    req: UpdateProfileRequest,
) -> Result<UserProfile, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, _inner| {
        users::update_profile(conn, req.email.as_deref(), req.username.as_deref(), req.first_name.as_deref(), req.last_name.as_deref())
    })
    .map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollTotpRequest {
    pub secret: String,
    pub totp_code: String,
}

#[tauri::command]
pub fn enroll_totp(
    app: AppHandle,
    state: State<'_, AppState>,
    req: EnrollTotpRequest,
) -> Result<SecondFactorStatus, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    if !verify_totp_code(&req.secret, &req.totp_code)
        .map_err(|e| String::from(e))?
    {
        return Err(String::from(AppError::message("AUTH_FAILED", "invalid TOTP code")));
    }

    let value_key = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?
    };

    let enc = encrypt_totp_secret(&value_key, &req.secret)?;
    meta::write_totp_secret_enc(&enc)?;

    session::with_db(&state, |conn, _inner| {
        users::set_totp_enrolled(conn, &enc)?;
        users::set_active_second_factor(conn, "totp")?;
        meta::update_second_factor_type("totp")?;
        Ok(read_status(conn)?)
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn enroll_biometric(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SecondFactorStatus, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;
    biometry::verify_user("Register fingerprint / Windows Hello for Argus", &app)?;

    session::with_db(&state, |conn, _inner| {
        users::set_biometric_enrolled(conn, true)?;
        let user = users::get_user(conn)?;
        if !user.totp_enabled {
            users::set_active_second_factor(conn, "biometric")?;
            meta::update_second_factor_type("biometric")?;
        }
        Ok(read_status(conn)?)
    })
    .map_err(|e| String::from(e))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetActiveSecondFactorRequest {
    pub second_factor_type: String,
}

#[tauri::command]
pub fn set_active_second_factor(
    app: AppHandle,
    state: State<'_, AppState>,
    req: SetActiveSecondFactorRequest,
) -> Result<SecondFactorStatus, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    let typ = req.second_factor_type.to_lowercase();
    session::with_db(&state, |conn, _inner| {
        users::set_active_second_factor(conn, &typ)?;
        meta::update_second_factor_type(&typ)?;
        Ok(read_status(conn)?)
    })
    .map_err(|e| String::from(e))
}

fn read_status(conn: &rusqlite::Connection) -> AppResult<SecondFactorStatus> {
    let user = users::get_user(conn)?;
    Ok(SecondFactorStatus {
        active_second_factor: user.second_factor_type,
        totp_enrolled: user.totp_enabled,
        biometric_enrolled: user.biometric_enrolled,
    })
}
