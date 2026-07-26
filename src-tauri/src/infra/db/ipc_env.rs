use std::collections::HashMap;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::infra::db::bucket_mappings;
use crate::infra::db::buckets;
use crate::infra::db::secrets;
use crate::error::{AppError, AppResult};
use crate::messages;

pub fn is_socket_injectable(secret_type: &str) -> bool {
    matches!(
        secret_type,
        "api_key" | "access_token" | "connection_string"
    )
}

pub fn plain_from_value(value: &Value) -> AppResult<String> {
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
    resolve_bucket_env_inner(conn, bucket_id, value_key, false)
}

/// Same as [`resolve_bucket_env`] but always injects real secret values
/// (used by `argus run --no-proxy` where there is no MITM rewrite).
pub fn resolve_bucket_env_real(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
) -> AppResult<HashMap<String, String>> {
    resolve_bucket_env_inner(conn, bucket_id, value_key, true)
}

fn resolve_bucket_env_inner(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
    force_real_secrets: bool,
) -> AppResult<HashMap<String, String>> {
    let bucket = buckets::get_bucket_meta(conn, bucket_id)?;
    let mappings = bucket_mappings::list_mappings(conn, bucket_id, value_key)?;
    let mut env = HashMap::new();
    for m in mappings {
        let use_proxy = !force_real_secrets && bucket.proxy_enabled && m.proxy_enabled;
        if use_proxy {
            if let Some(ph) = m.proxy_placeholder {
                env.insert(m.env_label, ph);
            }
            continue;
        }
        match m.mapping_type.as_str() {
            "text" => {
                if let Some(val) = m.text_value {
                    env.insert(m.env_label, val);
                }
            }
            _ => {
                let secret_type = match m.secret_type.as_deref() {
                    Some(t) => t,
                    None => continue,
                };
                if !is_socket_injectable(secret_type) {
                    continue;
                }
                let secret_id = match m.secret_id.as_deref() {
                    Some(id) => id,
                    None => continue,
                };
                let detail = secrets::get_secret_detail(conn, secret_id, value_key)?;
                let plain = plain_from_value(&detail.value)?;
                env.insert(m.env_label, plain);
            }
        }
    }
    Ok(env)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
    pub ca_bundle_path: String,
}

pub fn resolve_proxy_config(
    conn: &Connection,
    bucket_id: &str,
    client_token: &str,
) -> AppResult<Option<ProxyConfig>> {
    let meta = buckets::get_bucket_meta(conn, bucket_id)?;
    if !meta.proxy_enabled {
        return Ok(None);
    }
    let port = meta.proxy_port.ok_or_else(|| {
        AppError::message("PROXY_ERROR", messages::proxy_port_missing(&meta.name))
    })?;
    let encoded = urlencoding::encode(client_token);
    let base = format!("http://{encoded}@127.0.0.1:{port}");
    let ca_bundle_path = crate::infra::db::argus_dir()
        .join("ca-bundle.pem")
        .to_string_lossy()
        .into_owned();
    Ok(Some(ProxyConfig {
        enabled: true,
        http_proxy: base.clone(),
        https_proxy: base,
        no_proxy: "localhost,127.0.0.1,::1".to_string(),
        ca_bundle_path,
    }))
}
