use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::Value;

use crate::db::bucket_mappings;
use crate::db::secrets;
use crate::error::{AppError, AppResult};

pub fn is_socket_injectable(secret_type: &str) -> bool {
    matches!(
        secret_type,
        "api_key" | "access_token" | "connection_string"
    )
}

fn plain_from_value(value: &Value) -> AppResult<String> {
    let obj = value
        .as_object()
        .ok_or_else(|| AppError::message("DB_ERROR", "invalid secret value"))?;
    if let Some(v) = obj.get("value").and_then(|v| v.as_str()) {
        return Ok(v.to_string());
    }
    if let Some(v) = obj.get("password").and_then(|v| v.as_str()) {
        return Ok(v.to_string());
    }
    if let Some(v) = obj.get("apiKey").and_then(|v| v.as_str()) {
        return Ok(v.to_string());
    }
    if let Some(v) = obj.get("note").and_then(|v| v.as_str()) {
        return Ok(v.to_string());
    }
    if let Some((_, v)) = obj.iter().next() {
        if let Some(s) = v.as_str() {
            return Ok(s.to_string());
        }
    }
    Err(AppError::message("DB_ERROR", "secret value is empty"))
}

pub fn resolve_bucket_env(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
) -> AppResult<HashMap<String, String>> {
    let mappings = bucket_mappings::list_mappings(conn, bucket_id)?;
    let mut env = HashMap::new();
    for m in mappings {
        if !is_socket_injectable(&m.secret_type) {
            continue;
        }
        let detail = secrets::get_secret_detail(conn, &m.secret_id, value_key)?;
        let plain = plain_from_value(&detail.value)?;
        env.insert(m.env_label, plain);
    }
    Ok(env)
}
