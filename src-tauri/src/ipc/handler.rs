use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::infra::db::{buckets, client_grants, ipc_env};
use crate::ipc::peer::VerifiedClient;
use crate::ipc::protocol::{
    parse_incoming, IpcRequest, IpcResponse, ParsedIpcRequest, ProxyConfigPayload,
    SandboxCreateRequest, SandboxListRequest, SandboxRegisterPidsRequest, SandboxRevokeRequest,
    SandboxSessionInfo,
};
use crate::messages;
use crate::proxy::ProxyRuntime;
use crate::sandbox::approval::{request_client_grant, GrantInsertDetails, GrantRequest};
use crate::sandbox::cache::lookup_session_by_pid;
use crate::sandbox::service::{
    create_sandbox_session, list_active_sessions, register_session_pids, revoke_sandbox_session,
};
use crate::sessions::PendingApprovalStore;
use crate::state::AppState;

pub async fn handle_request(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    peer: &VerifiedClient,
    line: &str,
) -> String {
    let parsed = match parse_incoming(line) {
        Ok(p) => p,
        Err(e) => {
            return IpcResponse::Error {
                request_id: String::new(),
                code: "INVALID_REQUEST".into(),
                message: messages::invalid_request_json(&e.to_string()),
            }
            .to_line();
        }
    };

    let request_id = match &parsed {
        ParsedIpcRequest::FetchEnv(r) => r.request_id.clone(),
        ParsedIpcRequest::SandboxCreate(r) => r.request_id.clone(),
        ParsedIpcRequest::SandboxRegisterPids(r) => r.request_id.clone(),
        ParsedIpcRequest::SandboxRevoke(r) => r.request_id.clone(),
        ParsedIpcRequest::SandboxList(r) => r.request_id.clone(),
        ParsedIpcRequest::Unknown { request_id, .. } => request_id.clone(),
    };

    let state = app.state::<AppState>();
    let signed_in = {
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
        inner.is_signed_in()
    };

    if !signed_in {
        return IpcResponse::Locked {
            request_id,
            message: messages::locked_signed_out().into(),
        }
        .to_line();
    }

    let result = match parsed {
        ParsedIpcRequest::FetchEnv(req) => {
            process_fetch_env(app, pending_store, &state, peer, req).await
        }
        ParsedIpcRequest::SandboxCreate(req) => {
            process_sandbox_create(app, pending_store, &state, peer, req).await
        }
        ParsedIpcRequest::SandboxRegisterPids(req) => {
            process_sandbox_register_pids(&state, peer, req).await
        }
        ParsedIpcRequest::SandboxRevoke(req) => process_sandbox_revoke(&state, req).await,
        ParsedIpcRequest::SandboxList(req) => process_sandbox_list(&state, req).await,
        ParsedIpcRequest::Unknown { request_id, msg_type } => Ok(IpcResponse::Error {
            request_id,
            code: "INVALID_REQUEST".into(),
            message: format!("unknown IPC type: {msg_type}"),
        }),
    };

    match result {
        Ok(resp) => resp.to_line(),
        Err(e) => {
            let code = e.code().to_string();
            if code == "APPROVAL_DENIED" || code == "APPROVAL_TIMEOUT" {
                IpcResponse::Denied {
                    request_id,
                    code,
                    message: e.to_string(),
                }
                .to_line()
            } else {
                IpcResponse::Error {
                    request_id,
                    code,
                    message: e.to_string(),
                }
                .to_line()
            }
        }
    }
}

async fn process_fetch_env(
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

        if let Some(session) = lookup_session_by_pid(conn, &req.bucket_id, peer.pid)? {
            let env = ipc_env::resolve_bucket_env(conn, &req.bucket_id, value_key)?;
            let _ = session;
            return Ok((meta.name, meta.access_ttl_minutes, Some(env)));
        }

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
        let proxy = proxy_payload(state, &req.bucket_id, &req.client_token)?;
        return Ok(IpcResponse::Ok {
            request_id,
            env,
            proxy,
            session_id: None,
            proxy_port: None,
            expires_at: None,
            ca_bundle_path: None,
            sessions: None,
        });
    }

    request_client_grant(
        app,
        pending_store,
        state,
        GrantRequest {
            request_id: &request_id,
            bucket_id: &req.bucket_id,
            bucket_name: &bucket_name,
            client_token: &req.client_token,
            fingerprint,
            access_ttl,
            peer,
            client_label: None,
            grant_details: GrantInsertDetails {
                cwd: Some(peer.cwd.clone()),
                exe_path: Some(peer.exe_path.clone()),
                git_remote: peer.git_remote.clone(),
                run_args: Some(peer.run_args.clone()),
            },
        },
    )
    .await?;

    let env = with_session_db(state, |conn, value_key| {
        ipc_env::resolve_bucket_env(conn, &req.bucket_id, value_key)
    })?;
    touch_activity(state);
    let proxy = proxy_payload(state, &req.bucket_id, &req.client_token)?;
    Ok(IpcResponse::Ok {
        request_id,
        env,
        proxy,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
        sessions: None,
    })
}

async fn process_sandbox_create(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    state: &tauri::State<'_, AppState>,
    peer: &VerifiedClient,
    req: SandboxCreateRequest,
) -> Result<IpcResponse, AppError> {
    let request_id = req.request_id.clone();
    let fingerprint = peer.fingerprint.clone();
    let _token_hash = buckets::hash_token(&req.client_token);

    let proxy_check = with_session_db(state, |conn, _| {
        let meta = buckets::verify_client_token(conn, &req.bucket_id, &req.client_token)?;
        if !meta.proxy_enabled {
            return Err(AppError::message(
                "PROXY_DISABLED",
                messages::proxy_disabled(&meta.name),
            ));
        }
        let port = meta.proxy_port.ok_or_else(|| {
            AppError::message("PROXY_DISABLED", messages::proxy_port_missing(&meta.name))
        })?;
        Ok((meta.name.clone(), meta.access_ttl_minutes, port))
    })?;

    let (bucket_name, access_ttl, proxy_port) = proxy_check;

    let grant_id = request_client_grant(
        app,
        pending_store,
        state,
        GrantRequest {
            request_id: &request_id,
            bucket_id: &req.bucket_id,
            bucket_name: &bucket_name,
            client_token: &req.client_token,
            fingerprint: &fingerprint,
            access_ttl,
            peer,
            client_label: Some("argus run"),
            grant_details: GrantInsertDetails {
                cwd: Some(peer.cwd.clone()),
                exe_path: Some(peer.exe_path.clone()),
                git_remote: peer.git_remote.clone(),
                run_args: req.command_preview.clone(),
            },
        },
    )
    .await?;

    let (session, env, ca_bundle_path) = with_session_db(state, |conn, value_key| {
        let ttl = client_grants::access_ttl_minutes(conn, access_ttl)?;
        create_sandbox_session(
            conn,
            value_key,
            &req.bucket_id,
            &grant_id,
            &fingerprint,
            req.command_preview.as_deref(),
            ttl,
            peer.pid,
            proxy_port,
            &req.client_token,
        )
    })?;

    let _ = ProxyRuntime::ensure_bucket_running(app, &req.bucket_id, proxy_port);
    touch_activity(state);

    Ok(IpcResponse::Ok {
        request_id,
        session_id: Some(session.id),
        proxy_port: Some(proxy_port),
        expires_at: Some(session.expires_at),
        env,
        ca_bundle_path: Some(ca_bundle_path),
        proxy: None,
        sessions: None,
    })
}

async fn process_sandbox_register_pids(
    state: &tauri::State<'_, AppState>,
    peer: &VerifiedClient,
    req: SandboxRegisterPidsRequest,
) -> Result<IpcResponse, AppError> {
    with_session_db(state, |conn, _| register_session_pids(conn, &req.session_id, &req.pids))?;
    let _ = peer;
    Ok(IpcResponse::Ok {
        request_id: req.request_id,
        env: Default::default(),
        proxy: None,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
        sessions: None,
    })
}

async fn process_sandbox_revoke(
    state: &tauri::State<'_, AppState>,
    req: SandboxRevokeRequest,
) -> Result<IpcResponse, AppError> {
    with_session_db(state, |conn, _| revoke_sandbox_session(conn, &req.session_id))?;
    Ok(IpcResponse::Ok {
        request_id: req.request_id,
        env: Default::default(),
        proxy: None,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
        sessions: None,
    })
}

async fn process_sandbox_list(
    state: &tauri::State<'_, AppState>,
    req: SandboxListRequest,
) -> Result<IpcResponse, AppError> {
    let sessions = with_session_db(state, |conn, _| list_active_sessions(conn))?;
    let sessions = sessions
        .into_iter()
        .map(|s| SandboxSessionInfo {
            session_id: s.session_id,
            bucket_id: s.bucket_id,
            command_preview: s.command_preview,
            expires_at: s.expires_at,
            pids: s.pids,
        })
        .collect();
    Ok(IpcResponse::Ok {
        request_id: req.request_id,
        env: Default::default(),
        proxy: None,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
        sessions: Some(sessions),
    })
}

fn proxy_payload(
    state: &tauri::State<'_, AppState>,
    bucket_id: &str,
    client_token: &str,
) -> Result<Option<ProxyConfigPayload>, AppError> {
    with_session_db(state, |conn, _| {
        let cfg = ipc_env::resolve_proxy_config(conn, bucket_id, client_token)?;
        Ok(cfg.map(|c| ProxyConfigPayload {
            enabled: c.enabled,
            http_proxy: c.http_proxy,
            https_proxy: c.https_proxy,
            no_proxy: c.no_proxy,
            ca_bundle_path: c.ca_bundle_path,
        }))
    })
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
