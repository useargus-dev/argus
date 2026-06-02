use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretBox};

use crate::infra::db::DbPool;

#[derive(Clone, Debug)]
pub struct RegisterDraft {
    pub email: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    /// Master password kept only until finalize completes (memory-only).
    pub password: String,
    pub password_hash: String,
    pub second_factor_type: String,
    pub totp_secret_plain: Option<String>,
    pub biometric_enrolled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthScope {
    App,
    Vault,
    Buckets,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeStatus {
    pub app: bool,
    pub vault: bool,
    pub buckets: bool,
    pub vault_expires_at: Option<String>,
    pub buckets_expires_at: Option<String>,
}

pub struct AppState(pub Mutex<AppStateInner>);

impl Default for AppState {
    fn default() -> Self {
        AppState(Mutex::new(AppStateInner::default()))
    }
}

pub struct AppStateInner {
    pub db: Option<DbPool>,
    pub db_key: Option<SecretBox<[u8; 32]>>,
    pub value_key: Option<SecretBox<[u8; 32]>>,
    pub signed_in_at: Option<DateTime<Utc>>,
    pub vault_elevated_until: Option<DateTime<Utc>>,
    pub buckets_elevated_until: Option<DateTime<Utc>>,
    pub last_activity: AtomicU64,
    pub password_hash_cache: Option<String>,
    pub register_draft: Option<RegisterDraft>,
    /// Pending sign-in after password verified (identifier + password hash verified).
    pub pending_sign_in: Option<PendingSignIn>,
    /// Guards against duplicate `register_finalize` (e.g. React Strict Mode double mount).
    pub register_finalize_running: bool,
    pub auth_failures: u32,
    pub auth_lockout_until: Option<DateTime<Utc>>,
    /// UI / policy lock while keys remain in memory (re-unlock with second factor only).
    pub app_locked: bool,
}

#[derive(Clone, Debug)]
pub struct PendingSignIn {
    pub identifier: String,
    pub password: String,
    pub second_factor_type: String,
}

impl Default for AppStateInner {
    fn default() -> Self {
        Self {
            db: None,
            db_key: None,
            value_key: None,
            signed_in_at: None,
            vault_elevated_until: None,
            buckets_elevated_until: None,
            last_activity: AtomicU64::new(now_epoch()),
            password_hash_cache: None,
            register_draft: None,
            pending_sign_in: None,
            register_finalize_running: false,
            auth_failures: 0,
            auth_lockout_until: None,
            app_locked: false,
        }
    }
}

impl AppStateInner {
    pub fn touch_activity(&self) {
        self.last_activity
            .store(now_epoch(), Ordering::SeqCst);
    }

    /// Session is active when value keys are in memory (db pool may be temporarily
    /// detached during `with_db` queries).
    pub fn is_signed_in(&self) -> bool {
        self.value_key.is_some()
    }

    pub fn has_app_scope(&self) -> bool {
        self.is_signed_in() && !self.app_locked
    }

    pub fn soft_lock(&mut self) {
        self.app_locked = true;
        self.vault_elevated_until = None;
        self.buckets_elevated_until = None;
    }

    /// Vault access follows app unlock (no separate vault TTL).
    pub fn has_vault_scope(&self) -> bool {
        self.has_app_scope()
    }

    /// Bucket / tray admin follows app unlock (no separate buckets TTL).
    pub fn has_buckets_scope(&self) -> bool {
        self.has_app_scope()
    }

    pub fn scope_status(&self) -> ScopeStatus {
        ScopeStatus {
            app: self.has_app_scope(),
            vault: self.has_vault_scope(),
            buckets: self.has_buckets_scope(),
            vault_expires_at: None,
            buckets_expires_at: None,
        }
    }

    pub fn value_key(&self) -> Option<[u8; 32]> {
        self.value_key.as_ref().map(|k| *k.expose_secret())
    }

    pub fn clear_session(&mut self) {
        self.db = None;
        self.db_key = None;
        self.value_key = None;
        self.signed_in_at = None;
        self.vault_elevated_until = None;
        self.buckets_elevated_until = None;
        self.password_hash_cache = None;
        self.pending_sign_in = None;
        self.register_finalize_running = false;
        self.auth_failures = 0;
        self.auth_lockout_until = None;
        self.app_locked = false;
    }
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
