//! In-memory PID → sandbox session cache for transparent gate hot path.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};

use crate::error::AppResult;
use crate::infra::db::sandbox_sessions::SandboxSession;

#[derive(Debug, Clone)]
struct CachedSession {
    session: SandboxSession,
}

static CACHE: LazyLock<Mutex<SessionCache>> = LazyLock::new(|| Mutex::new(SessionCache::default()));

#[derive(Default)]
pub struct SessionCache {
    by_pid: HashMap<u32, CachedSession>,
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

    pub fn invalidate_session(&mut self, session_id: &str) {
        self.by_pid
            .retain(|_, v| v.session.id != session_id);
    }

    pub fn invalidate_pids(&mut self, pids: &[u32]) {
        for pid in pids {
            self.by_pid.remove(pid);
        }
    }

    pub fn clear(&mut self) {
        self.by_pid.clear();
    }
}

fn session_active(session: &SandboxSession) -> bool {
    DateTime::parse_from_rfc3339(&session.expires_at)
        .map(|exp| exp > Utc::now())
        .unwrap_or(false)
}

/// Lookup active sandbox session for `pid` in `bucket_id`, cache first then DB fallback.
pub fn lookup_session_by_pid(
    conn: &rusqlite::Connection,
    bucket_id: &str,
    pid: u32,
) -> AppResult<Option<SandboxSession>> {
    if let Ok(guard) = SessionCache::global().lock() {
        if let Some(session) = guard.lookup(bucket_id, pid) {
            return Ok(Some(session));
        }
    }

    let session = crate::infra::db::sandbox_sessions::lookup_active_session_by_pid(conn, pid)?;
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

pub fn warm_cache(session: SandboxSession, pids: &[u32]) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.insert_session(session, pids);
    }
}

pub fn invalidate_session_cache(session_id: &str) {
    if let Ok(mut guard) = SessionCache::global().lock() {
        guard.invalidate_session(session_id);
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
        assert!(cache.lookup("bucket-1", 100).is_some());
        assert!(cache.lookup("other", 100).is_none());
        cache.invalidate_session("sess_test");
        assert!(cache.lookup("bucket-1", 100).is_none());
    }
}
