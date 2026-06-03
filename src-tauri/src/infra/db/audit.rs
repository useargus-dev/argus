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
