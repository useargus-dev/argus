//! In-memory PID → sandbox session cache for transparent gate hot path.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};

use crate::error::AppResult;
use crate::infra::db::sandbox_sessions::{self, SandboxSession};

#[derive(Debug, Clone)]
struct CachedSession {
    session: SandboxSession,
}

static CACHE: LazyLock<Mutex<SessionCache>> = LazyLock::new(|| Mutex::new(SessionCache::default()));

#[derive(Default)]
pub struct SessionCache {
    by_pid: HashMap<u32, CachedSession>,
    relay_secrets: HashMap<String, [u8; 32]>,
    relay_nonces: HashMap<String, u64>,
    bucket_sessions: HashMap<String, Vec<String>>,
}

impl SessionCache {
    pub fn global() -> &'static Mutex<Self> {
        &CACHE
    }

    pub fn insert_session(&mut self, session: SandboxSession, pids: &[u32]) {
        for &pid in pids {
            self.by_pid.insert(
                pid,
                CachedSession {
                    session: session.clone(),
                },
            );
        }
        let entry = self
            .bucket_sessions
            .entry(session.bucket_id.clone())
            .or_default();
        if !entry.iter().any(|id| id == &session.id) {
            entry.push(session.id.clone());
        }
    }

    pub fn set_relay_secret(&mut self, session_id: &str, secret: [u8; 32]) {
        self.relay_secrets.insert(session_id.to_string(), secret);
        self.relay_nonces.remove(session_id);
    }

    pub fn lookup(&self, bucket_id: &str, pid: u32) -> Option<SandboxSession> {
        let entry = self.by_pid.get(&pid)?;
        if entry.session.bucket_id != bucket_id {
            return None;
        }
        if !session_active(&entry.session) {
            return None;
        }
        Some(entry.session.clone())
    }

    pub fn lookup_relay_secret(&self, bucket_id: &str, pid: u32) -> Option<[u8; 32]> {
        let entry = self.by_pid.get(&pid)?;
        if entry.session.bucket_id != bucket_id {
            return None;
        }
        self.relay_secrets.get(&entry.session.id).copied()
    }

    /// Accept relay nonce if strictly greater than the last seen nonce for this session.
    pub fn consume_relay_nonce(&mut self, session_id: &str, nonce: u64) -> bool {
        let last = self.relay_nonces.get(session_id).copied().unwrap_or(0);
        if nonce <= last {
            return false;
        }
        self.relay_nonces.insert(session_id.to_string(), nonce);
        true
    }

    pub fn invalidate_session(&mut self, session_id: &str) {
        self.by_pid
            .retain(|_, v| v.session.id != session_id);
        self.relay_secrets.remove(session_id);
        self.relay_nonces.remove(session_id);
        for ids in self.bucket_sessions.values_mut() {
            ids.retain(|id| id != session_id);
        }
    }

    pub fn invalidate_pids(&mut self, pids: &[u32]) {
        for pid in pids {
            self.by_pid.remove(pid);
        }
    }

    pub fn invalidate_sessions_for_bucket(&mut self, bucket_id: &str) {
        if let Some(ids) = self.bucket_sessions.remove(bucket_id) {
            for id in ids {
                self.invalidate_session(&id);
            }
        }
        self.by_pid.retain(|_, v| v.session.bucket_id != bucket_id);
    }

    pub fn clear(&mut self) {
        self.by_pid.clear();
        self.relay_secrets.clear();
        self.relay_nonces.clear();
        self.bucket_sessions.clear();
    }
}

fn session_active(session: &SandboxSession) -> bool {
    DateTime::parse_from_rfc3339(&session.expires_at)
        .map(|exp| exp > Utc::now())
        .unwrap_or(false)
}

fn revalidate_cached_session(
    conn: &rusqlite::Connection,
    bucket_id: &str,
    pid: u32,
    cached: &SandboxSession,
) -> AppResult<Option<SandboxSession>> {
    let db = sandbox_sessions::lookup_active_session_by_pid(conn, pid)?;
    match db {
        Some(s) if s.id == cached.id && s.bucket_id == bucket_id => Ok(Some(s)),
        _ => {
            invalidate_pids(&[pid]);
            Ok(None)
        }
    }
}

/// Lookup active sandbox session for `pid` in `bucket_id`, cache first then DB fallback.
pub fn lookup_session_by_pid(
    conn: &rusqlite::Connection,
    bucket_id: &str,
    pid: u32,
) -> AppResult<Option<SandboxSession>> {
    if let Ok(guard) = SessionCache::global().lock() {
        if let Some(session) = guard.lookup(bucket_id, pid) {
            drop(guard);
            return revalidate_cached_session(conn, bucket_id, pid, &session);
        }
    }

    let session = sandbox_sessions::lookup_active_session_by_pid(conn, pid)?;
    if let Some(ref s) = session {
        if s.bucket_id == bucket_id {
            if let Ok(mut guard) = SessionCache::global().lock() {
                guard.insert_session(s.clone(), &[pid]);
            }
        } else {
            return Ok(None);
        }
    }
    Ok(session.filter(|s| s.bucket_id == bucket_id))
}

pub fn lookup_relay_secret_by_pid(
    conn: &rusqlite::Connection,
    bucket_id: &str,
    pid: u32,
) -> AppResult<Option<[u8; 32]>> {
    if lookup_session_by_pid(conn, bucket_id, pid)?.is_none() {
        return Ok(None);
    }
    if let Ok(guard) = SessionCache::global().lock() {
        return Ok(guard.lookup_relay_secret(bucket_id, pid));
    }
    Ok(None)
}

/// Record a relay nonce for `session_id`; returns false on replay.
pub fn consume_relay_nonce(session_id: &str, nonce: u64) -> bool {
    SessionCache::global()
        .lock()
        .map(|mut guard| guard.consume_relay_nonce(session_id, nonce))
        .unwrap_or(false)
}

pub fn warm_cache(session: SandboxSession, pids: &[u32]) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.insert_session(session, pids);
    }
}

pub fn set_relay_secret(session_id: &str, secret: [u8; 32]) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.set_relay_secret(session_id, secret);
    }
}

pub fn invalidate_session_cache(session_id: &str) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.invalidate_session(session_id);
    }
}

pub fn invalidate_pids(pids: &[u32]) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.invalidate_pids(pids);
    }
}

pub fn invalidate_sessions_for_bucket(bucket_id: &str) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.invalidate_sessions_for_bucket(bucket_id);
    }
}

pub fn invalidate_revoked_sessions(revoked: &[(String, Vec<u32>)]) {
    for (session_id, pids) in revoked {
        invalidate_session_cache(session_id);
        invalidate_pids(pids);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_and_invalidate() {
        let mut cache = SessionCache::default();
        let session = SandboxSession {
            id: "sess_test".into(),
            bucket_id: "bucket-1".into(),
            grant_id: "g1".into(),
            parent_fingerprint: "fp".into(),
            command_preview: None,
            root_pid: Some(100),
            created_at: Utc::now().to_rfc3339(),
            expires_at: (Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        };
        cache.insert_session(session.clone(), &[100]);
        cache.set_relay_secret("sess_test", [9u8; 32]);
        assert!(cache.lookup("bucket-1", 100).is_some());
        assert!(cache.lookup_relay_secret("bucket-1", 100).is_some());
        assert!(cache.lookup("other", 100).is_none());
        cache.invalidate_session("sess_test");
        assert!(cache.lookup("bucket-1", 100).is_none());
        assert!(cache.lookup_relay_secret("bucket-1", 100).is_none());
    }

    #[test]
    fn relay_nonce_monotonic_rejects_replay() {
        let mut cache = SessionCache::default();
        assert!(cache.consume_relay_nonce("sess_a", 1));
        assert!(cache.consume_relay_nonce("sess_a", 2));
        assert!(!cache.consume_relay_nonce("sess_a", 2));
        assert!(!cache.consume_relay_nonce("sess_a", 1));
        assert!(cache.consume_relay_nonce("sess_a", 3));
    }

    #[test]
    fn relay_nonce_resets_on_session_invalidate() {
        let mut cache = SessionCache::default();
        cache.set_relay_secret("sess_b", [1u8; 32]);
        assert!(cache.consume_relay_nonce("sess_b", 10));
        cache.invalidate_session("sess_b");
        assert!(cache.consume_relay_nonce("sess_b", 1));
    }
}
