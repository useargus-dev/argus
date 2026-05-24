use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::db::bucket_mappings::{self, BucketMapping};
use crate::db::buckets::{self, BucketMeta, BucketWithToken};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::util::session;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBucketRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMappingRequest {
    pub bucket_id: String,
    pub env_label: String,
    pub secret_id: String,
}

fn require_buckets(inner: &crate::state::AppStateInner) -> AppResult<()> {
    if !inner.has_buckets_scope() {
        return Err(AppError::message(
            "APP_LOCKED",
            "unlock the app to manage buckets",
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn list_buckets(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<BucketMeta>, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        buckets::list_buckets(conn).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn create_bucket(
    app: AppHandle,
    state: State<'_, AppState>,
    req: CreateBucketRequest,
) -> Result<BucketWithToken, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        buckets::create_bucket(
            conn,
            &vk,
            &req.name,
            req.description.as_deref(),
        )
        .map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn delete_bucket(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        buckets::delete_bucket(conn, &id).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn set_bucket_active(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    active: bool,
) -> Result<BucketWithToken, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        buckets::set_bucket_active(conn, &vk, &id, active).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn list_bucket_mappings(
    app: AppHandle,
    state: State<'_, AppState>,
    bucket_id: String,
) -> Result<Vec<BucketMapping>, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        bucket_mappings::list_mappings(conn, &bucket_id).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn upsert_bucket_mapping(
    app: AppHandle,
    state: State<'_, AppState>,
    req: UpsertMappingRequest,
) -> Result<BucketMapping, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        bucket_mappings::upsert_mapping(
            conn,
            &req.bucket_id,
            &req.env_label,
            &req.secret_id,
        )
        .map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn delete_bucket_mapping(
    app: AppHandle,
    state: State<'_, AppState>,
    mapping_id: String,
) -> Result<(), String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        bucket_mappings::delete_mapping(conn, &mapping_id).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}

#[tauri::command]
pub fn get_bucket_token(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    session::touch_and_check_auto_lock(&app, &state, true).map_err(|e| String::from(e))?;

    session::with_db(&state, |conn, inner| {
        session::sync_scopes(&app, inner);
        require_buckets(inner)?;
        let vk = inner
            .value_key()
            .ok_or_else(|| AppError::message("NOT_SIGNED_IN", "not signed in"))?;
        buckets::get_bucket_token(conn, &vk, &id).map_err(|e| e.into())
    })
    .map_err(|e| String::from(e))
}
