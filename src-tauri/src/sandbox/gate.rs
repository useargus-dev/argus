//! Shared sandbox PID authorization (grant + boot ID) for fetch_env and transparent paths.

use rusqlite::Connection;

use crate::infra::db::{client_grants, sandbox_sessions};
use crate::sandbox::cache::{invalidate_pids, lookup_session_by_pid};
use crate::util::process_identity::process_boot_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidVerifyFailure {
    NotRegistered,
    GrantInactive,
    BootIdMismatch,
}

/// Verify `pid` is registered on an active sandbox session for `bucket_id` with live boot ID and grant.
pub fn verify_registered_pid(
    conn: &Connection,
    bucket_id: &str,
    pid: u32,
) -> Result<sandbox_sessions::SandboxSession, PidVerifyFailure> {
    let session = lookup_session_by_pid(conn, bucket_id, pid)
        .map_err(|_| PidVerifyFailure::NotRegistered)?
        .ok_or(PidVerifyFailure::NotRegistered)?;

    if !client_grants::grant_is_active(conn, &session.grant_id)
        .unwrap_or(false)
    {
        return Err(PidVerifyFailure::GrantInactive);
    }

    verify_pid_boot_id(conn, &session, pid)?;
    Ok(session)
}

fn verify_pid_boot_id(
    conn: &Connection,
    session: &sandbox_sessions::SandboxSession,
    pid: u32,
) -> Result<(), PidVerifyFailure> {
    let stored_boot = sandbox_sessions::get_pid_boot_id(conn, &session.id, pid)
        .unwrap_or(None);
    let live_boot = process_boot_id(pid).ok();
    let boot_ok = matches!(
        (stored_boot.as_deref(), live_boot.as_deref()),
        (Some(stored), Some(live)) if stored == live
    );
    if boot_ok {
        return Ok(());
    }
    if stored_boot.is_some() {
        let _ = sandbox_sessions::delete_stale_pid(conn, &session.id, pid);
        invalidate_pids(&[pid]);
    }
    Err(PidVerifyFailure::BootIdMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::meta::run_migrations;
    use crate::sandbox::cache::SessionCache;
    use chrono::Utc;
    use rusqlite::Connection;
    use uuid::Uuid;

    fn mem_conn() -> Connection {
        if let Ok(mut cache) = SessionCache::global().lock() {
            cache.clear();
        }
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn seed(conn: &Connection) -> (String, String) {
        let bucket_id = Uuid::new_v4().to_string();
        let grant_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        conn.execute(
            "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc, access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts, created_at, updated_at) VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, '[]', ?2, ?2)",
            rusqlite::params![bucket_id, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO client_grants (id, bucket_id, fingerprint, token_hash, granted_at, expires_at, last_seen_at) VALUES (?1, ?2, 'fp', 'hash', ?3, ?4, ?3)",
            rusqlite::params![grant_id, bucket_id, now, exp],
        )
        .unwrap();
        (bucket_id, grant_id)
    }

    #[test]
    fn verify_accepts_live_boot_id() {
        let conn = mem_conn();
        let (bucket_id, grant_id) = seed(&conn);
        let pid = std::process::id();
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            &grant_id,
            "fp",
            None,
            60,
        )
        .unwrap();
        let boot_id = process_boot_id(pid).unwrap();
        sandbox_sessions::register_pids(&conn, &session.id, &[(pid, boot_id)]).unwrap();
        assert!(verify_registered_pid(&conn, &bucket_id, pid).is_ok());
    }

    #[test]
    fn verify_rejects_boot_id_mismatch() {
        let conn = mem_conn();
        let (bucket_id, grant_id) = seed(&conn);
        let pid = std::process::id();
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            &grant_id,
            "fp",
            None,
            60,
        )
        .unwrap();
        sandbox_sessions::register_pids(&conn, &session.id, &[(pid, "stale".into())]).unwrap();
        assert!(matches!(
            verify_registered_pid(&conn, &bucket_id, pid),
            Err(PidVerifyFailure::BootIdMismatch)
        ));
    }
}
