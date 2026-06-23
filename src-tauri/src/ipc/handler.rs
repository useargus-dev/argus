use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::timeout;

use crate::error::AppError;
use crate::infra::db::{audit, buckets, client_grants, ipc_env, sandbox_sessions};
use crate::ipc::peer::VerifiedClient;
use crate::ipc::protocol::{
    parse_incoming, IpcRequest, IpcResponse, ParsedIpcRequest, ProxyConfigPayload,
    SandboxCreateRequest, SandboxRegisterPidsRequest, SandboxRevokeRequest,
};
use crate::messages;
use crate::proxy::ProxyRuntime;
use crate::sessions::{ClientAccessRequestEvent, PendingApprovalStore, PendingDecision};
use crate::state::AppState;

const APPROVAL_WAIT_SECS: u64 = 120;

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
        ParsedIpcRequest::Unknown { request_id, msg_type } => Ok(IpcResponse::Error {
            request_id,
            code: "INVALID_REQUEST".into(),
            message: format!("unknown IPC type: {msg_type}"),
        }),
    };

    match result {
        Ok(resp) => resp.to_line(),
        Err(e) => IpcResponse::Error {
            request_id,
            code: e.code().to_string(),
            message: e.to_string(),
        }
        .to_line(),
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
        });
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
                code: "APPROVAL_TIMEOUT".into(),
                message: messages::approval_timeout().into(),
            });
        }
    };

    match decision {
        PendingDecision::Deny => Ok(IpcResponse::Denied {
            request_id,
            code: "APPROVAL_DENIED".into(),
            message: messages::approval_denied().into(),
        }),
        PendingDecision::Accept { ttl_minutes } => {
            let ttl = if ttl_minutes > 0 {
                ttl_minutes
            } else {
                access_ttl
            };
            let details = client_grants::GrantDetails {
                cwd: Some(peer.cwd.clone()),
                exe_path: Some(peer.exe_path.clone()),
                git_remote: peer.git_remote.clone(),
                run_args: Some(peer.run_args.clone()),
            };
            let env = with_session_db(state, |conn, value_key| {
                client_grants::insert_grant(
                    conn,
                    &req.bucket_id,
                    fingerprint,
                    &req.client_token,
                    ttl,
                    Some(&peer.process_name),
                    Some(&details),
                )?;
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
            })
        }
    }
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
    let token_hash = buckets::hash_token(&req.client_token);

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

    let grant_id = ensure_grant_for_sandbox(
        app,
        pending_store,
        state,
        peer,
        &req,
        &bucket_name,
        access_ttl,
        &fingerprint,
        &token_hash,
    )
    .await?;

    let (session, env, ca_bundle_path) = with_session_db(state, |conn, value_key| {
        let ttl = client_grants::access_ttl_minutes(conn, access_ttl)?;
        let session = sandbox_sessions::create_session(
            conn,
            &req.bucket_id,
            &grant_id,
            &fingerprint,
            req.command_preview.as_deref(),
            ttl,
        )?;
        sandbox_sessions::register_pids(conn, &session.id, &[peer.pid])?;
        let env = ipc_env::resolve_bucket_env(conn, &req.bucket_id, value_key)?;
        let ca_bundle_path = ipc_env::resolve_proxy_config(conn, &req.bucket_id, &req.client_token)?
            .map(|c| c.ca_bundle_path)
            .unwrap_or_else(|| {
                crate::infra::db::argus_dir()
                    .join("ca-bundle.pem")
                    .to_string_lossy()
                    .into_owned()
            });
        audit::sandbox_session_created(
            conn,
            &req.bucket_id,
            &session.id,
            req.command_preview.as_deref(),
            proxy_port,
        )?;
        audit::sandbox_pid_registered(conn, &req.bucket_id, &session.id, &[peer.pid])?;
        Ok((session, env, ca_bundle_path))
    })?;

    let _ = ProxyRuntime::sync_enabled_buckets(app);
    touch_activity(state);

    Ok(IpcResponse::Ok {
        request_id,
        session_id: Some(session.id),
        proxy_port: Some(proxy_port),
        expires_at: Some(session.expires_at),
        env,
        ca_bundle_path: Some(ca_bundle_path),
        proxy: None,
    })
}

async fn ensure_grant_for_sandbox(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    state: &tauri::State<'_, AppState>,
    peer: &VerifiedClient,
    req: &SandboxCreateRequest,
    bucket_name: &str,
    access_ttl: i64,
    fingerprint: &str,
    token_hash: &str,
) -> Result<String, AppError> {
    let existing = with_session_db(state, |conn, _| {
        client_grants::find_active_grant(conn, &req.bucket_id, fingerprint, token_hash)
    })?;
    if let Some(grant) = existing {
        with_session_db(state, |conn, _| client_grants::touch_grant(conn, &grant.id))?;
        return Ok(grant.id);
    }

    let event = ClientAccessRequestEvent {
        request_id: req.request_id.clone(),
        bucket_id: req.bucket_id.clone(),
        bucket_name: bucket_name.to_string(),
        fingerprint: fingerprint.to_string(),
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
    let _ = app.emit("client-access-requested", event_clone);
    show_requests_window(app);

    let decision = match timeout(Duration::from_secs(APPROVAL_WAIT_SECS), rx).await {
        Ok(Ok(d)) => d,
        Ok(Err(_)) => PendingDecision::Deny,
        Err(_) => {
            pending_store.respond(&req.request_id, PendingDecision::Deny);
            return Err(AppError::message(
                "GRANT_REQUIRED",
                messages::approval_timeout(),
            ));
        }
    };

    match decision {
        PendingDecision::Deny => Err(AppError::message(
            "GRANT_REQUIRED",
            messages::approval_denied(),
        )),
        PendingDecision::Accept { ttl_minutes } => {
            let ttl = if ttl_minutes > 0 {
                ttl_minutes
            } else {
                access_ttl
            };
            let details = client_grants::GrantDetails {
                cwd: Some(peer.cwd.clone()),
                exe_path: Some(peer.exe_path.clone()),
                git_remote: peer.git_remote.clone(),
                run_args: req.command_preview.clone(),
            };
            let grant_id = with_session_db(state, |conn, _| {
                client_grants::insert_grant(
                    conn,
                    &req.bucket_id,
                    fingerprint,
                    &req.client_token,
                    ttl,
                    Some("argus run"),
                    Some(&details),
                )
            })?;
            Ok(grant_id)
        }
    }
}

async fn process_sandbox_register_pids(
    state: &tauri::State<'_, AppState>,
    peer: &VerifiedClient,
    req: SandboxRegisterPidsRequest,
) -> Result<IpcResponse, AppError> {
    with_session_db(state, |conn, _| {
        let session = sandbox_sessions::get_session(conn, &req.session_id)?
            .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
        sandbox_sessions::register_pids(conn, &req.session_id, &req.pids)?;
        audit::sandbox_pid_registered(conn, &session.bucket_id, &req.session_id, &req.pids)?;
        Ok(())
    })?;
    let _ = peer;
    Ok(IpcResponse::Ok {
        request_id: req.request_id,
        env: Default::default(),
        proxy: None,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
    })
}

async fn process_sandbox_revoke(
    state: &tauri::State<'_, AppState>,
    req: SandboxRevokeRequest,
) -> Result<IpcResponse, AppError> {
    with_session_db(state, |conn, _| {
        let session = sandbox_sessions::get_session(conn, &req.session_id)?
            .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
        if !sandbox_sessions::revoke_session(conn, &req.session_id)? {
            return Err(AppError::message(
                "SESSION_NOT_FOUND",
                "sandbox session not found",
            ));
        }
        audit::sandbox_session_revoked(conn, &session.bucket_id, &req.session_id)?;
        Ok(())
    })?;
    Ok(IpcResponse::Ok {
        request_id: req.request_id,
        env: Default::default(),
        proxy: None,
        session_id: None,
        proxy_port: None,
        expires_at: None,
        ca_bundle_path: None,
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

fn show_requests_window(app: &AppHandle) {
    crate::show_requests_window(app);
}
