use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::db::secrets::{self, SecretDetail, SecretMeta};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::util::session;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretWriteRequest {
    pub name: String,
    pub secret_type: String,
    pub organization: Option<String>,
    pub environment: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub expires_at: Option<String>,
    pub value: Value,
}

#[tauri::command]
pub fn search_secrets(
    app: AppHandle,
    state: State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<SecretMeta>, String> {
    run_search(&app, &state, query).map_err(|e| String::from(e))
}

#[tauri::command]
pub fn get_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<SecretDetail, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        session::require_vault_scope(inner, conn)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        secrets::get_secret_detail(conn, &id, &vk).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn create_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    req: SecretWriteRequest,
) -> Result<SecretMeta, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        session::require_vault_scope(inner, conn)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        let tags = req.tags.unwrap_or_default();
        secrets::create_secret(
            conn,
            &vk,
            &req.name,
            &req.secret_type,
            req.organization.as_deref(),
            req.environment.as_deref(),
            req.description.as_deref(),
            &tags,
            req.expires_at.as_deref(),
            &req.value,
        )
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn update_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    req: SecretWriteRequest,
) -> Result<SecretMeta, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        session::require_vault_scope(inner, conn)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        let tags = req.tags.unwrap_or_default();
        secrets::update_secret(
            conn,
            &vk,
            &id,
            &req.name,
            &req.secret_type,
            req.organization.as_deref(),
            req.environment.as_deref(),
            req.description.as_deref(),
            &tags,
            req.expires_at.as_deref(),
            &req.value,
        )
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn delete_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        session::require_vault_scope(inner, conn)?;
        secrets::delete_secret(conn, &id)
    })
    .map_err(|e| String::from(e))
}

fn run_search(
    app: &AppHandle,
    state: &State<'_, AppState>,
    query: Option<String>,
) -> AppResult<Vec<SecretMeta>> {
    session::touch_and_check_auto_lock(app, state, true)?;

    session::with_db(state, |conn, inner| {
        session::sync_scopes(app, inner);
        session::require_vault_scope(inner, conn)?;
        secrets::search_secrets(conn, query.as_deref())
    })
}
