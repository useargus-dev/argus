use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::state::{AppState, ScopeStatus};
use crate::util::session;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevateVaultRequest {
    pub totp_code: Option<String>,
    pub use_biometric: Option<bool>,
}

/// Legacy command: vault access is granted whenever the app is unlocked.
#[tauri::command]
pub fn elevate_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    _req: ElevateVaultRequest,
) -> Result<ScopeStatus, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;
    let scopes = session::scope_status_after_sync(&app, &state).map_err(|e| String::from(e))?;
    if !scopes.vault {
        return Err(String::from(AppError::message(
            "APP_LOCKED",
            "unlock the app first",
        )));
    }
    Ok(scopes)
}

/// Locks the whole app (same as idle app lock); vault has no separate lock.
#[tauri::command]
pub fn lock_vault(app: AppHandle, state: State<'_, AppState>) -> Result<ScopeStatus, String> {
    session::soft_lock_app(&app, &state).map_err(|e| String::from(e))
}
