//! Sandbox session operations (DB + cache).

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::infra::db::{audit, ipc_env, sandbox_sessions};
use crate::sandbox::cache::{invalidate_session_cache, warm_cache};

#[derive(Debug, Clone)]
pub struct ActiveSessionInfo {
    pub session_id: String,
    pub bucket_id: String,
    pub command_preview: Option<String>,
    pub expires_at: String,
    pub pids: Vec<u32>,
}

pub fn create_sandbox_session(
    conn: &Connection,
    value_key: &[u8; 32],
    bucket_id: &str,
    grant_id: &str,
    parent_fingerprint: &str,
    command_preview: Option<&str>,
    ttl_minutes: i64,
    cli_pid: u32,
    proxy_port: u16,
    client_token: &str,
) -> AppResult<(sandbox_sessions::SandboxSession, std::collections::HashMap<String, String>, String)> {
    let session = sandbox_sessions::create_session(
        conn,
        bucket_id,
        grant_id,
        parent_fingerprint,
        command_preview,
        ttl_minutes,
    )?;
    sandbox_sessions::register_pids(conn, &session.id, &[cli_pid])?;
    warm_cache(session.clone(), &[cli_pid]);

    let env = ipc_env::resolve_bucket_env(conn, bucket_id, value_key)?;
    let ca_bundle_path = ipc_env::resolve_proxy_config(conn, bucket_id, client_token)?
        .map(|c| c.ca_bundle_path)
        .unwrap_or_else(|| {
            crate::infra::db::argus_dir()
                .join("ca-bundle.pem")
                .to_string_lossy()
                .into_owned()
        });

    audit::sandbox_session_created(conn, bucket_id, &session.id, command_preview, proxy_port)?;
    audit::sandbox_pid_registered(conn, bucket_id, &session.id, &[cli_pid])?;

    Ok((session, env, ca_bundle_path))
}

pub fn register_session_pids(
    conn: &Connection,
    session_id: &str,
    pids: &[u32],
) -> AppResult<sandbox_sessions::SandboxSession> {
    let session = sandbox_sessions::get_session(conn, session_id)?
        .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
    sandbox_sessions::register_pids(conn, session_id, pids)?;
    warm_cache(session.clone(), pids);
    audit::sandbox_pid_registered(conn, &session.bucket_id, session_id, pids)?;
    Ok(session)
}

pub fn revoke_sandbox_session(conn: &Connection, session_id: &str) -> AppResult<()> {
    let session = sandbox_sessions::get_session(conn, session_id)?
        .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
    if !sandbox_sessions::revoke_session(conn, session_id)? {
        return Err(AppError::message(
            "SESSION_NOT_FOUND",
            "sandbox session not found",
        ));
    }
    invalidate_session_cache(session_id);
    audit::sandbox_session_revoked(conn, &session.bucket_id, session_id)?;
    Ok(())
}

pub fn list_active_sessions(conn: &Connection) -> AppResult<Vec<ActiveSessionInfo>> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.bucket_id, s.command_preview, s.expires_at
             FROM sandbox_sessions s
             WHERE s.revoked_at IS NULL
               AND datetime(s.expires_at) > datetime('now')
             ORDER BY s.created_at DESC",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let mut out = Vec::new();
    for row in rows {
        let (session_id, bucket_id, command_preview, expires_at) =
            row.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let pids = list_session_pids(conn, &session_id)?;
        out.push(ActiveSessionInfo {
            session_id,
            bucket_id,
            command_preview,
            expires_at,
            pids,
        });
    }
    Ok(out)
}

fn list_session_pids(conn: &Connection, session_id: &str) -> AppResult<Vec<u32>> {
    let mut stmt = conn
        .prepare("SELECT pid FROM sandbox_session_pids WHERE session_id = ?1 ORDER BY pid")
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([session_id], |r| r.get::<_, i64>(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(rows
        .filter_map(|r| r.ok())
        .map(|p| p as u32)
        .collect())
}
