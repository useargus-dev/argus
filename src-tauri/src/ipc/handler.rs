use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::timeout;

use crate::db::{buckets, client_grants, ipc_env};
use crate::error::AppError;
use crate::ipc::peer::VerifiedClient;
use crate::ipc::protocol::{IpcRequest, IpcResponse};
use crate::sessions::{ClientAccessRequestEvent, PendingApprovalStore, PendingDecision};
use crate::state::AppState;

const APPROVAL_WAIT_SECS: u64 = 120;

pub async fn handle_request(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    peer: &VerifiedClient,
    line: &str,
) -> String {
    let req: IpcRequest = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(e) => {
            return IpcResponse::Error {
                request_id: String::new(),
                code: "INVALID_REQUEST".into(),
                message: format!("invalid JSON: {e}"),
            }
            .to_line();
        }
    };

    let request_id = req.request_id.clone();

    let state = app.state::<AppState>();
    let session_ok = {
        let inner = match state.0.lock() {
            Ok(g) => g,
            Err(_) => {
                return IpcResponse::Error {
                    request_id,
                    code: "LOCK_ERROR".into(),
                    message: "state poisoned".into(),
                }
                .to_line();
            }
        };
        inner.is_signed_in() && inner.has_app_scope()
    };

    if !session_ok {
        let msg = if state
            .0
            .lock()
            .map(|i| i.is_signed_in() && i.app_locked)
            .unwrap_or(false)
        {
            "Argus is locked — unlock the app to approve access"
        } else {
            "Argus is not signed in"
        };
        return IpcResponse::Locked {
            request_id,
            message: msg.into(),
        }
        .to_line();
    }

    match process_request(app, pending_store, &state, peer, req).await {
        Ok(resp) => resp.to_line(),
        Err(e) => {
            let code = e.code().to_string();
            IpcResponse::Error {
                request_id,
                code,
                message: e.to_string(),
            }
            .to_line()
        }
    }
}

async fn process_request(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    state: &tauri::State<'_, AppState>,
    peer: &VerifiedClient,
    req: IpcRequest,
) -> Result<IpcResponse, AppError> {
    let request_id = req.request_id.clone();
    let fingerprint = &peer.fingerprint;
    let token_hash = buckets::hash_token(&req.client_token);

    let (bucket_name, access_ttl, existing_env) = with_session_db(state, |conn, value_key| {
        let meta = buckets::verify_client_token(conn, &req.bucket_id, &req.client_token)?;
        let ttl = client_grants::access_ttl_minutes(conn, meta.access_ttl_minutes)?;

        if let Some(grant) =
            client_grants::find_active_grant(conn, &req.bucket_id, fingerprint, &token_hash)?
        {
            client_grants::touch_grant(conn, &grant.id)?;
            let env = ipc_env::resolve_bucket_env(conn, &req.bucket_id, value_key)?;
            return Ok((meta.name, ttl, Some(env)));
        }
        Ok((meta.name, ttl, None))
    })?;

    if let Some(env) = existing_env {
        touch_activity(state);
        return Ok(IpcResponse::Ok { request_id, env });
    }

    let event = ClientAccessRequestEvent {
        request_id: request_id.clone(),
        bucket_id: req.bucket_id.clone(),
        bucket_name,
        fingerprint: fingerprint.clone(),
        pid: peer.pid,
        exe_path: peer.exe_path.clone(),
        cwd: peer.cwd.clone(),
        cwd_verified: peer.cwd_verified,
        run_args: peer.run_args.clone(),
        git_remote: peer.git_remote.clone(),
        process_name: peer.process_name.clone(),
        machine_id: peer.machine_id.clone(),
        access_ttl_minutes: access_ttl,
        created_at: Utc::now().to_rfc3339(),
    };

    let (rx, event_clone) = pending_store.register(event);
    let _ = app.emit("client-access-requested", event_clone.clone());
    show_requests_window(app);

    let decision = match timeout(Duration::from_secs(APPROVAL_WAIT_SECS), rx).await {
        Ok(Ok(d)) => d,
        Ok(Err(_)) => PendingDecision::Deny,
        Err(_) => {
            pending_store.respond(&request_id, PendingDecision::Deny);
            return Ok(IpcResponse::Denied {
                request_id,
                message: "approval timed out".into(),
            });
        }
    };

    match decision {
        PendingDecision::Deny => Ok(IpcResponse::Denied {
            request_id,
            message: "access denied by user".into(),
        }),
        PendingDecision::Accept { ttl_minutes } => {
            let ttl = if ttl_minutes > 0 { ttl_minutes } else { access_ttl };
            let env = with_session_db(state, |conn, value_key| {
                client_grants::insert_grant(
                    conn,
                    &req.bucket_id,
                    fingerprint,
                    &req.client_token,
                    ttl,
                    Some(&peer.process_name),
                )?;
                ipc_env::resolve_bucket_env(conn, &req.bucket_id, value_key)
            })?;
            touch_activity(state);
            Ok(IpcResponse::Ok { request_id, env })
        }
    }
}

fn with_session_db<T, F>(state: &tauri::State<'_, AppState>, f: F) -> Result<T, AppError>
where
    F: FnOnce(&rusqlite::Connection, &[u8; 32]) -> Result<T, AppError>,
{
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    let pool = inner
        .db
        .as_ref()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let value_key = inner
        .value_key()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let conn = pool
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
    f(&conn, &value_key)
}

fn touch_activity(state: &tauri::State<'_, AppState>) {
    if let Ok(inner) = state.0.lock() {
        inner.touch_activity();
    }
}

fn show_requests_window(app: &AppHandle) {
    crate::show_requests_window(app);
}
