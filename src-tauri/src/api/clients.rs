use std::sync::Arc;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::infra::db::client_grants::{self, GrantRow};
use crate::ipc::IpcRuntime;
use crate::sandbox::lifecycle::revoke_sessions_for_grant;
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

fn require_signed_in(state: &State<'_, AppState>) -> Result<(), String> {
    let inner = state
        .0
        .lock()
        .map_err(|_| "state poisoned".to_string())?;
    if !inner.is_signed_in() {
        return Err("not signed in".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn is_signed_in(state: State<'_, AppState>) -> Result<bool, String> {
    let inner = state.0.lock().map_err(|_| "state poisoned".to_string())?;
    Ok(inner.is_signed_in())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn list_pending(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ClientAccessRequestEvent>, String> {
    require_signed_in(&state)?;
    Ok(pending_store(&app).list())
}

#[tauri::command]
pub fn respond_access(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RespondClientAccessRequest,
) -> Result<(), String> {
    require_signed_in(&state)?;

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

    let _ = app.emit("client-access-resolved", &req.request_id);
    Ok(())
}

#[tauri::command]
pub fn pending_count(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    require_signed_in(&state)?;
    Ok(pending_store(&app).count() as u32)
}

#[tauri::command]
pub fn list_grants(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<GrantRow>, String> {
    session::touch_and_check_auto_lock(&app, &state, false).map_err(|e| String::from(e))?;
    let inner = state.0.lock().map_err(|_| "state poisoned".to_string())?;
    session::require_app_unlocked(&inner).map_err(|e| String::from(e))?;
    let pool = inner.db.as_ref().ok_or("not signed in")?;
    let conn = pool.lock().map_err(|_| "db poisoned".to_string())?;
    client_grants::list_grants(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn revoke_grant(
    app: AppHandle,
    state: State<'_, AppState>,
    grant_id: String,
) -> Result<(), String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;
    let inner = state.0.lock().map_err(|_| "state poisoned".to_string())?;
    session::require_app_unlocked(&inner).map_err(|e| String::from(e))?;
    let pool = inner.db.as_ref().ok_or("not signed in")?;
    let conn = pool.lock().map_err(|_| "db poisoned".to_string())?;
    revoke_sessions_for_grant(&conn, &grant_id).map_err(|e| e.to_string())?;
    client_grants::revoke_grant(&conn, &grant_id).map_err(|e| e.to_string())
}
