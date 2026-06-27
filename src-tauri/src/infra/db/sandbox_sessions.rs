use chrono::{Duration, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct SandboxSession {
    pub id: String,
    pub bucket_id: String,
    pub grant_id: String,
    pub parent_fingerprint: String,
    pub command_preview: Option<String>,
    pub root_pid: Option<i64>,
    pub created_at: String,
    pub expires_at: String,
}

pub fn create_session(
    conn: &Connection,
    bucket_id: &str,
    grant_id: &str,
    parent_fingerprint: &str,
    command_preview: Option<&str>,
    ttl_minutes: i64,
) -> AppResult<SandboxSession> {
    let id = format!("sess_{}", Uuid::new_v4());
    let now = Utc::now();
    let expires = now + Duration::minutes(ttl_minutes.max(1));
    let now_s = now.to_rfc3339();
    let exp_s = expires.to_rfc3339();

    conn.execute(
        "INSERT INTO sandbox_sessions (id, bucket_id, grant_id, parent_fingerprint, command_preview, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            bucket_id,
            grant_id,
            parent_fingerprint,
            command_preview,
            now_s,
            exp_s
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    Ok(SandboxSession {
        id,
        bucket_id: bucket_id.to_string(),
        grant_id: grant_id.to_string(),
        parent_fingerprint: parent_fingerprint.to_string(),
        command_preview: command_preview.map(str::to_string),
        root_pid: None,
        created_at: now_s,
        expires_at: exp_s,
    })
}

pub fn register_pids(
    conn: &Connection,
    session_id: &str,
    pid_boot_pairs: &[(u32, String)],
) -> AppResult<()> {
    if pid_boot_pairs.is_empty() {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    for (pid, boot_id) in pid_boot_pairs {
        if boot_id.is_empty() {
            return Err(AppError::message(
                "PROCESS_ID",
                format!("missing process boot id for pid {pid}"),
            ));
        }
        conn.execute(
            "INSERT INTO sandbox_session_pids (session_id, pid, process_boot_id, added_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(session_id, pid) DO UPDATE SET process_boot_id = excluded.process_boot_id",
            params![session_id, *pid as i64, boot_id, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    if let Some((first, _)) = pid_boot_pairs.first() {
        conn.execute(
            "UPDATE sandbox_sessions SET root_pid = COALESCE(root_pid, ?2) WHERE id = ?1",
            params![session_id, *first as i64],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    Ok(())
}

pub fn revoke_session(conn: &Connection, session_id: &str) -> AppResult<bool> {
    let now = Utc::now().to_rfc3339();
    let n = conn
        .execute(
            "UPDATE sandbox_sessions SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
            params![session_id, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(n > 0)
}

pub fn list_session_pids(conn: &Connection, session_id: &str) -> AppResult<Vec<u32>> {
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

pub fn get_session(conn: &Connection, session_id: &str) -> AppResult<Option<SandboxSession>> {
    let row = conn.query_row(
        "SELECT id, bucket_id, grant_id, parent_fingerprint, command_preview, root_pid, created_at, expires_at FROM sandbox_sessions WHERE id = ?1 AND revoked_at IS NULL AND datetime(expires_at) > datetime('now')",
        [session_id],
        |r| {
            Ok(SandboxSession {
                id: r.get(0)?,
                bucket_id: r.get(1)?,
                grant_id: r.get(2)?,
                parent_fingerprint: r.get(3)?,
                command_preview: r.get(4)?,
                root_pid: r.get(5)?,
                created_at: r.get(6)?,
                expires_at: r.get(7)?,
            })
        },
    );
    match row {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::message("DB_ERROR", e.to_string())),
    }
}

pub fn lookup_active_session_by_pid(
    conn: &Connection,
    pid: u32,
) -> AppResult<Option<SandboxSession>> {
    let row = conn.query_row(
        "SELECT s.id, s.bucket_id, s.grant_id, s.parent_fingerprint, s.command_preview, s.root_pid, s.created_at, s.expires_at FROM sandbox_sessions s INNER JOIN sandbox_session_pids p ON p.session_id = s.id WHERE p.pid = ?1 AND s.revoked_at IS NULL AND datetime(s.expires_at) > datetime('now') ORDER BY s.created_at DESC LIMIT 1",
        [pid as i64],
        |r| {
            Ok(SandboxSession {
                id: r.get(0)?,
                bucket_id: r.get(1)?,
                grant_id: r.get(2)?,
                parent_fingerprint: r.get(3)?,
                command_preview: r.get(4)?,
                root_pid: r.get(5)?,
                created_at: r.get(6)?,
                expires_at: r.get(7)?,
            })
        },
    );
    match row {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::message("DB_ERROR", e.to_string())),
    }
}

pub fn get_pid_boot_id(
    conn: &Connection,
    session_id: &str,
    pid: u32,
) -> AppResult<Option<String>> {
    let row: Result<String, _> = conn.query_row(
        "SELECT process_boot_id FROM sandbox_session_pids WHERE session_id = ?1 AND pid = ?2",
        params![session_id, pid as i64],
        |r| r.get(0),
    );
    match row {
        Ok(id) if id.is_empty() => Ok(None),
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::message("DB_ERROR", e.to_string())),
    }
}

pub fn delete_stale_pid(conn: &Connection, session_id: &str, pid: u32) -> AppResult<()> {
    conn.execute(
        "DELETE FROM sandbox_session_pids WHERE session_id = ?1 AND pid = ?2",
        params![session_id, pid as i64],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn revoke_sessions_by_grant_id(
    conn: &Connection,
    grant_id: &str,
) -> AppResult<Vec<(String, Vec<u32>)>> {
    let mut stmt = conn
        .prepare("SELECT id FROM sandbox_sessions WHERE grant_id = ?1 AND revoked_at IS NULL")
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let ids: Vec<String> = stmt
        .query_map([grant_id], |r| r.get(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut out = Vec::new();
    for session_id in ids {
        let pids = list_session_pids(conn, &session_id)?;
        if revoke_session(conn, &session_id)? {
            out.push((session_id, pids));
        }
    }
    Ok(out)
}

pub fn revoke_all_sessions_for_bucket(
    conn: &Connection,
    bucket_id: &str,
) -> AppResult<Vec<(String, Vec<u32>)>> {
    let mut stmt = conn
        .prepare("SELECT id FROM sandbox_sessions WHERE bucket_id = ?1 AND revoked_at IS NULL")
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let ids: Vec<String> = stmt
        .query_map([bucket_id], |r| r.get(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut out = Vec::new();
    for session_id in ids {
        let pids = list_session_pids(conn, &session_id)?;
        if revoke_session(conn, &session_id)? {
            out.push((session_id, pids));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::meta::run_migrations;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn seed_bucket(conn: &Connection) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc, access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts, created_at, updated_at) VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, '[]', ?2, ?2)",
            params![id, now],
        )
        .unwrap();
        id
    }

    #[test]
    fn session_create_register_lookup_revoke() {
        let conn = mem_conn();
        let bucket_id = seed_bucket(&conn);
        let session = create_session(
            &conn,
            &bucket_id,
            "grant-1",
            "fp-test",
            Some("uvicorn app:main"),
            60,
        )
        .unwrap();
        register_pids(
            &conn,
            &session.id,
            &[(1234, "boot-a".into()), (5678, "boot-b".into())],
        )
        .unwrap();
        let found = lookup_active_session_by_pid(&conn, 5678).unwrap().unwrap();
        assert_eq!(found.id, session.id);
        assert!(revoke_session(&conn, &session.id).unwrap());
        assert!(lookup_active_session_by_pid(&conn, 5678).unwrap().is_none());
    }
}
