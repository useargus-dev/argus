//! Shared DB access for sandbox IPC and approval flows.

use rusqlite::Connection;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

pub fn with_session_db<T, F>(state: &State<'_, AppState>, f: F) -> Result<T, AppError>
where
    F: FnOnce(&Connection, &[u8; 32]) -> Result<T, AppError>,
{
    let inner = state
        .0
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "state poisoned"))?;
    let pool = inner
        .db
        .as_ref()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let value_key = inner
        .value_key()
        .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
    let conn = pool
        .lock()
        .map_err(|_| AppError::message("LOCK_ERROR", "db poisoned"))?;
    f(&conn, &value_key)
}
