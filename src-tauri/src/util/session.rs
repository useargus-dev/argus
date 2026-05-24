use std::sync::atomic::Ordering;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

use crate::db::settings;
use crate::error::{AppError, AppResult};
use crate::state::{now_epoch, AppState, AppStateInner, ScopeStatus};

/// Soft-lock the app (keys stay in memory). Clears vault/bucket scopes.
pub fn soft_lock_app(app: &AppHandle, state: &tauri::State<'_, AppState>) -> AppResult<ScopeStatus> {
    let scopes = {
        let mut inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if !inner.is_signed_in() {
            return Err(AppError::message("NOT_SIGNED_IN", "not signed in"));
        }
        inner.soft_lock();
        inner.scope_status()
    };
    let _ = app.emit("app-locked", ());
    let _ = app.emit("scope-changed", scopes.clone());
    Ok(scopes)
}

pub fn require_app_unlocked(inner: &AppStateInner) -> AppResult<()> {
    if !inner.is_signed_in() {
        return Err(AppError::message("NOT_SIGNED_IN", "not signed in"));
    }
    if inner.app_locked {
        return Err(AppError::message(
            "APP_LOCKED",
            "app is locked — verify with your second factor",
        ));
    }
    Ok(())
}

/// Idle check for scope polling: may soft-lock without returning an error.
pub fn poll_idle_app_lock(app: &AppHandle, state: &tauri::State<'_, AppState>) -> AppResult<()> {
    let should_lock = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
        if !inner.is_signed_in() || inner.app_locked {
            return Ok(());
        }
        let minutes = read_auto_lock_minutes_for_inner(&inner)?;
        is_idle_expired(&inner, minutes)
    };
    if should_lock {
        soft_lock_app(app, state)?;
    }
    Ok(())
}

/// Check idle timeout and optionally reset the activity timer.
/// When `reset_activity` is false (e.g. scope polling), idle is evaluated without
/// extending the session — so background polls do not prevent auto-lock.
pub fn touch_and_check_auto_lock(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    reset_activity: bool,
) -> AppResult<()> {
    if !reset_activity {
        poll_idle_app_lock(app, state)?;
        return Ok(());
    }

    let should_lock = {
        let inner = state
            .0
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;

        if !inner.is_signed_in() {
            return Err(AppError::message("NOT_SIGNED_IN", "not signed in"));
        }
        require_app_unlocked(&inner)?;

        let minutes = read_auto_lock_minutes_for_inner(&inner)?;
        let expired = is_idle_expired(&inner, minutes);

        if !expired {
            inner.touch_activity();
        }

        expired
    };

    if should_lock {
        soft_lock_app(app, state)?;
        return Err(AppError::message(
            "APP_LOCKED",
            "app locked due to inactivity",
        ));
    }

    Ok(())
}

fn is_idle_expired(inner: &AppStateInner, minutes: u64) -> bool {
    if minutes == 0 {
        return false;
    }
    let last = inner.last_activity.load(Ordering::SeqCst);
    let now = now_epoch();
    now.saturating_sub(last) >= minutes * 60
}

/// Read auto-lock setting while caller already holds `state.0` (avoids re-entrant lock).
fn read_auto_lock_minutes_for_inner(inner: &AppStateInner) -> AppResult<u64> {
    let pool = inner
        .db
        .as_ref()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let conn = pool
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
    read_auto_lock_minutes_conn(&conn)
}

pub fn read_auto_lock_minutes(state: &tauri::State<'_, AppState>) -> AppResult<u64> {
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    read_auto_lock_minutes_for_inner(&inner)
}

fn read_auto_lock_minutes_conn(conn: &Connection) -> AppResult<u64> {
    let raw = settings::get_or_default(conn, "auto_lock_minutes", "30")?;
    raw.parse::<u64>()
        .map_err(|_| AppError::message("DB_ERROR", "invalid auto_lock_minutes"))
}

/// Vault and buckets scopes follow app unlock; no per-scope TTL expiry.
pub fn expire_scopes_if_needed(_inner: &mut AppStateInner) -> bool {
    false
}

pub fn sync_scopes(app: &AppHandle, inner: &mut AppStateInner) {
    if expire_scopes_if_needed(inner) {
        let scopes = inner.scope_status();
        let _ = app.emit("scope-changed", scopes);
    }
}

pub fn require_vault_scope(inner: &AppStateInner, _conn: &Connection) -> AppResult<()> {
    if !inner.has_vault_scope() {
        return Err(AppError::message(
            "APP_LOCKED",
            "unlock the app to access secrets",
        ));
    }
    Ok(())
}

pub fn with_db<F, T>(state: &tauri::State<'_, AppState>, f: F) -> AppResult<T>
where
    F: FnOnce(&Connection, &mut AppStateInner) -> AppResult<T>,
{
    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    let pool = inner
        .db
        .take()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let result = {
        let conn = pool
            .lock()
            .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
        f(&conn, &mut inner)
    };
    inner.db = Some(pool);
    result
}

pub fn scope_status_after_sync(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
) -> AppResult<ScopeStatus> {
    let mut inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    sync_scopes(app, &mut inner);
    Ok(inner.scope_status())
}
