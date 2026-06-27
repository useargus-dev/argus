//! Shared client grant approval flow for fetch_env and sandbox_create.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::time::timeout;

use crate::error::AppError;
use crate::infra::db::client_grants;
use crate::ipc::peer::VerifiedClient;
use crate::messages;
use crate::sandbox::db::with_session_db;
use crate::sessions::{ClientAccessRequestEvent, PendingApprovalStore, PendingDecision};
use crate::state::AppState;

const APPROVAL_WAIT_SECS: u64 = 120;

#[derive(Debug, Clone)]
pub struct GrantInsertDetails {
    pub cwd: Option<String>,
    pub exe_path: Option<String>,
    pub git_remote: Option<String>,
    pub run_args: Option<String>,
}

pub struct GrantRequest<'a> {
    pub request_id: &'a str,
    pub bucket_id: &'a str,
    pub bucket_name: &'a str,
    pub client_token: &'a str,
    pub fingerprint: &'a str,
    pub access_ttl: i64,
    pub peer: &'a VerifiedClient,
    pub client_label: Option<&'a str>,
    pub grant_details: GrantInsertDetails,
}

/// Return existing grant id or prompt user and insert a new grant.
pub async fn request_client_grant(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    state: &tauri::State<'_, AppState>,
    req: GrantRequest<'_>,
) -> Result<String, AppError> {
    let token_hash = crate::infra::db::buckets::hash_token(req.client_token);

    let existing = with_session_db(state, |conn, _| {
        client_grants::find_active_grant(conn, req.bucket_id, req.fingerprint, &token_hash)
    })?;
    if let Some(grant) = existing {
        with_session_db(state, |conn, _| client_grants::touch_grant(conn, &grant.id))?;
        return Ok(grant.id);
    }

    let event = ClientAccessRequestEvent {
        request_id: req.request_id.to_string(),
        bucket_id: req.bucket_id.to_string(),
        bucket_name: req.bucket_name.to_string(),
        fingerprint: req.fingerprint.to_string(),
        pid: req.peer.pid,
        exe_path: req.peer.exe_path.clone(),
        cwd: req.peer.cwd.clone(),
        cwd_verified: req.peer.cwd_verified,
        run_args: req
            .grant_details
            .run_args
            .clone()
            .unwrap_or_else(|| req.peer.run_args.clone()),
        git_remote: req.peer.git_remote.clone(),
        process_name: req.peer.process_name.clone(),
        machine_id: req.peer.machine_id.clone(),
        access_ttl_minutes: req.access_ttl,
        created_at: Utc::now().to_rfc3339(),
    };

    let (rx, event_clone) = pending_store.register(event);
    let _ = app.emit("client-access-requested", event_clone);
    crate::show_requests_window(app);

    let decision = match timeout(Duration::from_secs(APPROVAL_WAIT_SECS), rx).await {
        Ok(Ok(d)) => d,
        Ok(Err(_)) => PendingDecision::Deny,
        Err(_) => {
            pending_store.respond(req.request_id, PendingDecision::Deny);
            return Err(AppError::message(
                "APPROVAL_TIMEOUT",
                messages::approval_timeout(),
            ));
        }
    };

    match decision {
        PendingDecision::Deny => Err(AppError::message(
            "APPROVAL_DENIED",
            messages::approval_denied(),
        )),
        PendingDecision::Accept { ttl_minutes } => {
            let ttl = if ttl_minutes > 0 {
                ttl_minutes
            } else {
                req.access_ttl
            };
            let details = client_grants::GrantDetails {
                cwd: req.grant_details.cwd,
                exe_path: req.grant_details.exe_path,
                git_remote: req.grant_details.git_remote,
                run_args: req.grant_details.run_args,
            };
            let label = req
                .client_label
                .unwrap_or(&req.peer.process_name);
            with_session_db(state, |conn, _| {
                client_grants::insert_grant(
                    conn,
                    req.bucket_id,
                    req.fingerprint,
                    req.client_token,
                    ttl,
                    Some(label),
                    Some(&details),
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_run_args_prefers_details_over_peer() {
        let details = GrantInsertDetails {
            cwd: None,
            exe_path: None,
            git_remote: None,
            run_args: Some("uvicorn app:main".into()),
        };
        let peer_args = "node server.js".to_string();
        let merged = details
            .run_args
            .clone()
            .unwrap_or_else(|| peer_args.clone());
        assert_eq!(merged, "uvicorn app:main");
        let empty = GrantInsertDetails {
            run_args: None,
            cwd: None,
            exe_path: None,
            git_remote: None,
        };
        assert_eq!(
            empty.run_args.clone().unwrap_or_else(|| peer_args.clone()),
            "node server.js"
        );
    }
}
