//! Sandbox session operations (DB + cache).

use rand::RngCore;
use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::infra::db::{audit, client_grants, ipc_env, sandbox_sessions};
use crate::sandbox::cache::{
    invalidate_pids, invalidate_session_cache, set_relay_secret, warm_cache,
};
use crate::util::process_identity::process_boot_id;

const MAX_PIDS_PER_REGISTER: usize = 64;

#[derive(Debug, Clone)]
pub struct ActiveSessionInfo {
    pub session_id: String,
    pub bucket_id: String,
    pub command_preview: Option<String>,
    pub expires_at: String,
    pub pids: Vec<u32>,
}

fn session_forbidden() -> AppError {
    AppError::message("SESSION_FORBIDDEN", "sandbox session access denied")
}

fn assert_session_owner(session: &sandbox_sessions::SandboxSession, peer_fingerprint: &str) -> AppResult<()> {
    if session.parent_fingerprint != peer_fingerprint {
        return Err(session_forbidden());
    }
    Ok(())
}

pub fn session_grant_active(conn: &Connection, session: &sandbox_sessions::SandboxSession) -> AppResult<bool> {
    client_grants::grant_is_active(conn, &session.grant_id)
}

pub fn create_sandbox_session(
    conn: &Connection,
    value_key: &[u8; 32],
    bucket_id: &str,
    grant_id: &str,
    parent_fingerprint: &str,
    command_preview: Option<&str>,
    ttl_minutes: i64,
    proxy_port: u16,
    client_token: &str,
    inject_real_secrets: bool,
) -> AppResult<(
    sandbox_sessions::SandboxSession,
    std::collections::HashMap<String, String>,
    String,
    [u8; 32],
)> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let result: AppResult<_> = (|| {
        let session = sandbox_sessions::create_session(
            conn,
            bucket_id,
            grant_id,
            parent_fingerprint,
            command_preview,
            ttl_minutes,
        )?;

        let env = if inject_real_secrets {
            ipc_env::resolve_bucket_env_real(conn, bucket_id, value_key)?
        } else {
            ipc_env::resolve_bucket_env(conn, bucket_id, value_key)?
        };
        let ca_bundle_path = ipc_env::resolve_proxy_config(conn, bucket_id, client_token)?
            .map(|c| c.ca_bundle_path)
            .unwrap_or_else(|| {
                crate::infra::db::argus_dir()
                    .join("ca-bundle.pem")
                    .to_string_lossy()
                    .into_owned()
            });

        audit::sandbox_session_created(conn, bucket_id, &session.id, command_preview, proxy_port)?;

        Ok((session, env, ca_bundle_path))
    })();

    match result {
        Ok((session, env, ca_bundle_path)) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
            let mut relay_secret = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut relay_secret);
            warm_cache(session.clone(), &[]);
            set_relay_secret(&session.id, relay_secret);
            Ok((session, env, ca_bundle_path, relay_secret))
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

pub fn register_session_pids(
    conn: &Connection,
    peer_fingerprint: &str,
    session_id: &str,
    pids: &[u32],
) -> AppResult<sandbox_sessions::SandboxSession> {
    if pids.len() > MAX_PIDS_PER_REGISTER {
        return Err(AppError::message(
            "INVALID_REQUEST",
            format!("too many pids (max {MAX_PIDS_PER_REGISTER})"),
        ));
    }

    let session = sandbox_sessions::get_session(conn, session_id)?
        .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
    assert_session_owner(&session, peer_fingerprint)?;
    if !session_grant_active(conn, &session)? {
        return Err(AppError::message(
            "SESSION_FORBIDDEN",
            "grant expired or revoked",
        ));
    }

    let mut pairs = Vec::with_capacity(pids.len());
    for &pid in pids {
        let boot_id = process_boot_id(pid)?;
        pairs.push((pid, boot_id));
    }

    sandbox_sessions::register_pids(conn, session_id, &pairs)?;
    warm_cache(session.clone(), pids);
    audit::sandbox_pid_registered(conn, &session.bucket_id, session_id, pids)?;
    Ok(session)
}

pub fn revoke_sandbox_session(
    conn: &Connection,
    peer_fingerprint: &str,
    session_id: &str,
) -> AppResult<()> {
    let session = sandbox_sessions::get_session(conn, session_id)?
        .ok_or_else(|| AppError::message("SESSION_NOT_FOUND", "sandbox session not found"))?;
    assert_session_owner(&session, peer_fingerprint)?;
    let pids = sandbox_sessions::list_session_pids(conn, session_id)?;
    if !sandbox_sessions::revoke_session(conn, session_id)? {
        return Err(AppError::message(
            "SESSION_NOT_FOUND",
            "sandbox session not found",
        ));
    }
    invalidate_session_cache(session_id);
    invalidate_pids(&pids);
    audit::sandbox_session_revoked(conn, &session.bucket_id, session_id)?;
    Ok(())
}

pub fn list_active_sessions(
    conn: &Connection,
    parent_fingerprint: &str,
) -> AppResult<Vec<ActiveSessionInfo>> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.bucket_id, s.command_preview, s.expires_at
             FROM sandbox_sessions s
             WHERE s.revoked_at IS NULL
               AND s.parent_fingerprint = ?1
               AND datetime(s.expires_at) > datetime('now')
             ORDER BY s.created_at DESC",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let rows = stmt
        .query_map([parent_fingerprint], |r| {
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
        let pids = sandbox_sessions::list_session_pids(conn, &session_id)?;
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

pub fn revoke_all_sessions_for_bucket(
    conn: &Connection,
    bucket_id: &str,
) -> AppResult<Vec<(String, Vec<u32>)>> {
    sandbox_sessions::revoke_all_sessions_for_bucket(conn, bucket_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::meta::run_migrations;
    use chrono::Utc;
    use uuid::Uuid;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn seed_bucket_and_grant(conn: &Connection) -> (String, String) {
        let bucket_id = Uuid::new_v4().to_string();
        let grant_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        conn.execute(
            "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc,
             access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts,
             created_at, updated_at)
             VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, '[]', ?2, ?2)",
            rusqlite::params![bucket_id, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO client_grants (id, bucket_id, fingerprint, token_hash, granted_at, expires_at, last_seen_at)
             VALUES (?1, ?2, 'fp-owner', 'hash', ?3, ?4, ?3)",
            rusqlite::params![grant_id, bucket_id, now, exp],
        )
        .unwrap();
        (bucket_id, grant_id)
    }

    #[test]
    fn list_scoped_to_fingerprint() {
        let conn = mem_conn();
        let (bucket_id, grant_id) = seed_bucket_and_grant(&conn);
        let s1 = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            &grant_id,
            "fp-owner",
            None,
            60,
        )
        .unwrap();
        sandbox_sessions::create_session(&conn, &bucket_id, &grant_id, "fp-other", None, 60).unwrap();
        let list = list_active_sessions(&conn, "fp-owner").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_id, s1.id);
    }

    #[test]
    fn revoke_forbidden_wrong_fingerprint() {
        let conn = mem_conn();
        let (bucket_id, grant_id) = seed_bucket_and_grant(&conn);
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            &grant_id,
            "fp-owner",
            None,
            60,
        )
        .unwrap();
        assert!(revoke_sandbox_session(&conn, "fp-other", &session.id).is_err());
    }
}
