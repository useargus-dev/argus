use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::json;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub fn insert_audit(
    conn: &Connection,
    event_type: &str,
    actor: Option<&str>,
    target_id: Option<&str>,
    metadata: serde_json::Value,
) -> AppResult<()> {
    let id = Uuid::new_v4().to_string();
    let meta_str = serde_json::to_string(&metadata)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    conn.execute(
        "INSERT INTO audit_log (id, event_type, actor, target_id, metadata, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            event_type,
            actor,
            target_id,
            meta_str,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn proxy_request(
    conn: &Connection,
    bucket_id: &str,
    host: &str,
    path: &str,
    method: &str,
    env_label: Option<&str>,
    status: u16,
    latency_ms: u64,
    pid: u32,
    session_id: Option<&str>,
    capture_mode: &str,
) -> AppResult<()> {
    insert_audit(
        conn,
        "PROXY_REQUEST",
        Some("proxy"),
        Some(bucket_id),
        json!({
            "host": host,
            "path": path,
            "method": method,
            "env_label": env_label,
            "status": status,
            "latency_ms": latency_ms,
            "pid": pid,
            "session_id": session_id,
            "capture_mode": capture_mode,
        }),
    )
}

pub fn proxy_host_denied(conn: &Connection, bucket_id: &str, host: &str, pid: u32) -> AppResult<()> {
    insert_audit(
        conn,
        "PROXY_HOST_DENIED",
        Some("proxy"),
        Some(bucket_id),
        json!({ "host": host, "pid": pid }),
    )
}

pub fn proxy_grant_denied(conn: &Connection, bucket_id: &str, pid: u32) -> AppResult<()> {
    insert_audit(
        conn,
        "PROXY_GRANT_DENIED",
        Some("proxy"),
        Some(bucket_id),
        json!({ "pid": pid }),
    )
}

pub fn sandbox_session_created(
    conn: &Connection,
    bucket_id: &str,
    session_id: &str,
    command_preview: Option<&str>,
    proxy_port: u16,
) -> AppResult<()> {
    insert_audit(
        conn,
        "SANDBOX_SESSION_CREATED",
        Some("sandbox"),
        Some(bucket_id),
        json!({
            "session_id": session_id,
            "command_preview": command_preview,
            "proxy_port": proxy_port,
        }),
    )
}

pub fn sandbox_session_revoked(conn: &Connection, bucket_id: &str, session_id: &str) -> AppResult<()> {
    insert_audit(
        conn,
        "SANDBOX_SESSION_REVOKED",
        Some("sandbox"),
        Some(bucket_id),
        json!({ "session_id": session_id }),
    )
}

pub fn sandbox_pid_registered(
    conn: &Connection,
    bucket_id: &str,
    session_id: &str,
    pids: &[u32],
) -> AppResult<()> {
    insert_audit(
        conn,
        "SANDBOX_PID_REGISTERED",
        Some("sandbox"),
        Some(bucket_id),
        json!({ "session_id": session_id, "pids": pids }),
    )
}

pub fn sandbox_transparent_denied(
    conn: &Connection,
    bucket_id: &str,
    pid: u32,
    reason: &str,
) -> AppResult<()> {
    insert_audit(
        conn,
        "SANDBOX_TRANSPARENT_DENIED",
        Some("sandbox"),
        Some(bucket_id),
        json!({ "pid": pid, "reason": reason }),
    )
}
