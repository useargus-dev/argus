use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::ipc::IpcRuntime;
use crate::sessions::{ClientAccessRequestEvent, PendingApprovalStore, PendingDecision};
use crate::state::AppState;
use crate::util::session;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondClientAccessRequest {
    pub request_id: String,
    pub accept: bool,
    pub ttl_minutes: Option<i64>,
}

fn pending_store(app: &AppHandle) -> Arc<PendingApprovalStore> {
    app.state::<IpcRuntime>().pending()
}

#[tauri::command]
pub fn list_pending_client_access(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ClientAccessRequestEvent>, String> {
    session::touch_and_check_auto_lock(&app, &state, false).map_err(|e| String::from(e))?;
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    session::require_app_unlocked(&guard).map_err(|e| String::from(e))?;
    Ok(pending_store(&app).list())
}

#[tauri::command]
pub fn respond_to_client_access(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RespondClientAccessRequest,
) -> Result<(), String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    session::require_app_unlocked(&guard).map_err(|e| String::from(e))?;

    let decision = if req.accept {
        PendingDecision::Accept {
            ttl_minutes: req.ttl_minutes.unwrap_or(0),
        }
    } else {
        PendingDecision::Deny
    };

    if !pending_store(&app).respond(&req.request_id, decision) {
        return Err("pending request not found or already resolved".into());
    }
    Ok(())
}

#[tauri::command]
pub fn pending_client_access_count(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    session::touch_and_check_auto_lock(&app, &state, false).map_err(|e| String::from(e))?;
    Ok(pending_store(&app).count() as u32)
}
