use std::collections::HashSet;

use chrono::Utc;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const PROXY_TOKEN_LEN: usize = 24;
const PROXY_TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::infra::db::buckets;
use crate::infra::db::hosts::{allowed_hosts_to_json, host_is_allowed, parse_allowed_hosts_json};
use crate::infra::db::secrets;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketMapping {
    pub id: String,
    pub bucket_id: String,
    pub env_label: String,
    pub mapping_type: String,
    pub secret_id: Option<String>,
    pub secret_name: Option<String>,
    pub secret_type: Option<String>,
    pub text_value: Option<String>,
    pub proxy_enabled: bool,
    pub proxy_placeholder: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub created_at: String,
}

fn row_to_mapping(row: &rusqlite::Row<'_>) -> rusqlite::Result<BucketMapping> {
    Ok(BucketMapping {
        id: row.get(0)?,
        bucket_id: row.get(1)?,
        env_label: row.get(2)?,
        mapping_type: row.get(3)?,
        secret_id: row.get(4)?,
        secret_name: row.get(5)?,
        secret_type: row.get(6)?,
        text_value: row.get(7)?,
        proxy_enabled: row.get::<_, i64>(8)? != 0,
        proxy_placeholder: None,
        allowed_hosts: parse_allowed_hosts_json(&row.get::<_, String>(9)?).unwrap_or_default(),
        created_at: row.get(10)?,
    })
}

const MAPPING_SELECT: &str = r"
SELECT m.id, m.bucket_id, m.env_label, m.mapping_type, m.secret_id, s.name, s.secret_type,
       m.text_value, m.proxy_enabled, m.allowed_hosts, m.created_at
";

const LIST_SQL: &str = r"
SELECT m.id, m.bucket_id, m.env_label, m.mapping_type, m.secret_id, s.name, s.secret_type,
       m.text_value, m.proxy_enabled, m.allowed_hosts, m.created_at
FROM bucket_mappings m
LEFT JOIN secrets s ON s.id = m.secret_id AND s.is_archived = 0
WHERE m.bucket_id = ?1
ORDER BY m.env_label COLLATE NOCASE
";

fn decrypt_placeholder(value_key: &[u8; 32], enc: Option<String>) -> AppResult<Option<String>> {
    match enc {
        Some(e) if !e.is_empty() => {
            let plain = decrypt_value(value_key, &e)?;
            Ok(Some(String::from_utf8_lossy(&plain).into_owned()))
        }
        _ => Ok(None),
    }
}

pub fn generate_proxy_placeholder() -> String {
    let mut rng = rand::thread_rng();
    let token: String = (0..PROXY_TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..PROXY_TOKEN_CHARS.len());
            PROXY_TOKEN_CHARS[idx] as char
        })
        .collect();
    format!("argus-proxy-{token}")
}

/// Plaintext proxy tokens already assigned in this bucket (optionally excluding one mapping).
fn bucket_proxy_tokens(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
    exclude_mapping_id: Option<&str>,
) -> AppResult<HashSet<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, proxy_placeholder FROM bucket_mappings
             WHERE bucket_id = ?1 AND proxy_placeholder IS NOT NULL AND proxy_placeholder != ''",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([bucket_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let mut used = HashSet::new();
    for row in rows {
        let (id, enc) = row.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        if exclude_mapping_id == Some(id.as_str()) {
            continue;
        }
        if let Some(plain) = decrypt_placeholder(value_key, Some(enc))? {
            used.insert(plain);
        }
    }
    Ok(used)
}

fn generate_unique_proxy_placeholder(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
    exclude_mapping_id: Option<&str>,
) -> AppResult<String> {
    let used = bucket_proxy_tokens(conn, bucket_id, value_key, exclude_mapping_id)?;
    const MAX_ATTEMPTS: u32 = 64;
    for _ in 0..MAX_ATTEMPTS {
        let candidate = generate_proxy_placeholder();
        if !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(AppError::message(
        "PROXY_TOKEN_EXHAUSTED",
        "could not allocate a unique proxy token for this bucket",
    ))
}

fn encrypt_unique_proxy_placeholder(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
    exclude_mapping_id: Option<&str>,
) -> AppResult<String> {
    let plain = generate_unique_proxy_placeholder(conn, bucket_id, value_key, exclude_mapping_id)?;
    encrypt_value(value_key, plain.as_bytes())
}

/// Regenerate tokens for proxy-enabled mappings that collide within a bucket.
pub fn ensure_bucket_proxy_tokens_unique(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
) -> AppResult<()> {
    let mut stmt = conn
        .prepare(
            "SELECT id, proxy_placeholder FROM bucket_mappings
             WHERE bucket_id = ?1 AND proxy_enabled = 1
               AND proxy_placeholder IS NOT NULL AND proxy_placeholder != ''",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows: Vec<(String, String)> = stmt
        .query_map([bucket_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut seen = HashSet::new();
    for (id, enc) in rows {
        let Some(plain) = decrypt_placeholder(value_key, Some(enc.clone()))? else {
            continue;
        };
        if seen.contains(&plain) {
            let new_enc =
                encrypt_unique_proxy_placeholder(conn, bucket_id, value_key, Some(&id))?;
            conn.execute(
                "UPDATE bucket_mappings SET proxy_placeholder = ?2 WHERE id = ?1",
                params![id, new_enc],
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
            if let Some(new_plain) = decrypt_placeholder(value_key, Some(new_enc))? {
                seen.insert(new_plain);
            }
            continue;
        }
        seen.insert(plain);
    }
    Ok(())
}

pub fn list_mappings(conn: &Connection, bucket_id: &str, value_key: &[u8; 32]) -> AppResult<Vec<BucketMapping>> {
    ensure_bucket_proxy_tokens_unique(conn, bucket_id, value_key)?;

    let mut stmt = conn
        .prepare(LIST_SQL)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([bucket_id], row_to_mapping)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let mut result = Vec::new();
    for row in rows {
        let mut m = row.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        if m.mapping_type == "text" {
            if let Some(ref enc) = m.text_value {
                let plain = decrypt_value(value_key, enc)?;
                m.text_value = Some(String::from_utf8_lossy(&plain).into_owned());
            }
        }
        let enc_ph: Option<String> = conn
            .query_row(
                "SELECT proxy_placeholder FROM bucket_mappings WHERE id = ?1",
                [&m.id],
                |r| r.get(0),
            )
            .ok();
        m.proxy_placeholder = decrypt_placeholder(value_key, enc_ph)?;
        result.push(m);
    }
    Ok(result)
}

pub fn upsert_mapping(
    conn: &Connection,
    bucket_id: &str,
    env_label: &str,
    mapping_type: &str,
    secret_id: Option<&str>,
    text_value: Option<&str>,
    proxy_enabled: bool,
    allowed_hosts: &[String],
    value_key: &[u8; 32],
) -> AppResult<BucketMapping> {
    let bucket = buckets::get_bucket_meta(conn, bucket_id)?;
    let proxy_enabled = proxy_enabled && bucket.proxy_enabled;
    let allowed_hosts_json = allowed_hosts_to_json(allowed_hosts)?;
    let label = normalize_env_label(env_label)?;
    if label.is_empty() {
        return Err(AppError::message("VALIDATION_ERROR", "env name is required"));
    }

    let encrypted_text = match mapping_type {
        "secret" => {
            let sid = secret_id
                .ok_or_else(|| AppError::message("VALIDATION_ERROR", "secret_id is required for secret type"))?;
            secrets::get_secret_meta(conn, sid)?;
            None
        }
        "text" => {
            let val = text_value
                .filter(|v| !v.is_empty())
                .ok_or_else(|| AppError::message("VALIDATION_ERROR", "text value is required for text type"))?;
            Some(encrypt_value(value_key, val.as_bytes())?)
        }
        _ => {
            return Err(AppError::message("VALIDATION_ERROR", "mapping_type must be 'secret' or 'text'"));
        }
    };

    let existing: Option<(String, Option<String>, i64, String)> = conn
        .query_row(
            "SELECT id, proxy_placeholder, proxy_enabled, allowed_hosts FROM bucket_mappings WHERE bucket_id = ?1 AND env_label = ?2",
            params![bucket_id, label],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();

    ensure_bucket_proxy_tokens_unique(conn, bucket_id, value_key)?;

    let now = Utc::now().to_rfc3339();
    if let Some((id, existing_ph, _was_proxy, _existing_hosts_json)) = existing {
        let hosts_json = allowed_hosts_json;
        let proxy_placeholder = resolve_proxy_placeholder(
            conn,
            bucket_id,
            Some(&id),
            proxy_enabled,
            existing_ph.as_deref(),
            value_key,
        )?;
        conn.execute(
            "UPDATE bucket_mappings SET mapping_type = ?2, secret_id = ?3, text_value = ?4,
             proxy_enabled = ?5, proxy_placeholder = ?6, allowed_hosts = ?7 WHERE id = ?1",
            params![
                id,
                mapping_type,
                secret_id,
                encrypted_text,
                if proxy_enabled { 1 } else { 0 },
                proxy_placeholder,
                hosts_json
            ],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id, value_key)
    } else {
        let id = Uuid::new_v4().to_string();
        let proxy_placeholder = if proxy_enabled {
            Some(encrypt_unique_proxy_placeholder(
                conn,
                bucket_id,
                value_key,
                Some(&id),
            )?)
        } else {
            None
        };
        conn.execute(
            "INSERT INTO bucket_mappings (id, bucket_id, env_label, mapping_type, secret_id, text_value,
             proxy_enabled, proxy_placeholder, allowed_hosts, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                bucket_id,
                label,
                mapping_type,
                secret_id,
                encrypted_text,
                if proxy_enabled { 1 } else { 0 },
                proxy_placeholder,
                allowed_hosts_json,
                now
            ],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id, value_key)
    }
}

fn resolve_proxy_placeholder(
    conn: &Connection,
    bucket_id: &str,
    mapping_id: Option<&str>,
    proxy_enabled: bool,
    existing_enc: Option<&str>,
    value_key: &[u8; 32],
) -> AppResult<Option<String>> {
    if !proxy_enabled {
        return Ok(None);
    }
    if let Some(enc) = existing_enc.filter(|s| !s.is_empty()) {
        if let Some(mid) = mapping_id {
            if let Some(plain) = decrypt_placeholder(value_key, Some(enc.to_string()))? {
                let used = bucket_proxy_tokens(conn, bucket_id, value_key, Some(mid))?;
                if used.contains(&plain) {
                    return Ok(Some(encrypt_unique_proxy_placeholder(
                        conn, bucket_id, value_key, Some(mid),
                    )?));
                }
            }
        }
        Ok(Some(enc.to_string()))
    } else {
        Ok(Some(encrypt_unique_proxy_placeholder(
            conn,
            bucket_id,
            value_key,
            mapping_id,
        )?))
    }
}

pub fn delete_mapping(conn: &Connection, mapping_id: &str) -> AppResult<()> {
    let n = conn
        .execute("DELETE FROM bucket_mappings WHERE id = ?1", [mapping_id])
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    if n == 0 {
        return Err(AppError::message("NOT_FOUND", "mapping not found"));
    }
    Ok(())
}

/// Plaintext placeholders and secrets for proxy header rewriting (proxy-enabled mappings only).
pub struct ProxyRewriteEntry {
    pub env_label: String,
    pub placeholder: String,
    pub secret_plain: String,
}

/// True when at least one proxy-enabled mapping on this bucket allows the host.
pub fn bucket_allows_proxy_host(conn: &Connection, bucket_id: &str, host: &str) -> AppResult<bool> {
    let mut stmt = conn
        .prepare(
            "SELECT allowed_hosts FROM bucket_mappings WHERE bucket_id = ?1 AND proxy_enabled = 1",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([bucket_id], |r| r.get::<_, String>(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    for json in rows {
        let json = json.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let hosts = parse_allowed_hosts_json(&json)?;
        if host_is_allowed(host, &hosts) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn list_proxy_rewrite_entries(
    conn: &Connection,
    bucket_id: &str,
    value_key: &[u8; 32],
    host: &str,
) -> AppResult<Vec<ProxyRewriteEntry>> {
    use crate::infra::db::ipc_env::{is_socket_injectable, plain_from_value};

    let mut stmt = conn
        .prepare(
            r"SELECT m.id, m.env_label, m.mapping_type, m.secret_id, m.text_value, m.proxy_placeholder, m.allowed_hosts
              FROM bucket_mappings m
              WHERE m.bucket_id = ?1 AND m.proxy_enabled = 1",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([bucket_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let mut out = Vec::new();
    for row in rows {
        let (mapping_id, env_label, mapping_type, secret_id, text_enc, ph_enc, hosts_json) =
            row.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let allowed = parse_allowed_hosts_json(&hosts_json)?;
        if !host_is_allowed(host, &allowed) {
            continue;
        }
        let placeholder = match decrypt_placeholder(value_key, ph_enc)? {
            Some(p) => p,
            None => {
                generate_unique_proxy_placeholder(conn, bucket_id, value_key, Some(&mapping_id))?
            }
        };
        let secret_plain = match mapping_type.as_str() {
            "text" => {
                let enc = text_enc.ok_or_else(|| {
                    AppError::message("DB_ERROR", "missing text value for proxy mapping")
                })?;
                String::from_utf8_lossy(&decrypt_value(value_key, &enc)?).into_owned()
            }
            _ => {
                let sid = secret_id.ok_or_else(|| {
                    AppError::message("DB_ERROR", "missing secret for proxy mapping")
                })?;
                let detail = secrets::get_secret_detail(conn, &sid, value_key)?;
                let st = detail.meta.secret_type.as_str();
                if !is_socket_injectable(st) {
                    continue;
                }
                plain_from_value(&detail.value)?
            }
        };
        out.push(ProxyRewriteEntry {
            env_label,
            placeholder,
            secret_plain,
        });
    }
    Ok(out)
}

fn get_mapping(conn: &Connection, id: &str, value_key: &[u8; 32]) -> AppResult<BucketMapping> {
    let mut m = conn
        .query_row(
            &format!("{MAPPING_SELECT} FROM bucket_mappings m LEFT JOIN secrets s ON s.id = m.secret_id WHERE m.id = ?1"),
            [id],
            row_to_mapping,
        )
        .map_err(|_| AppError::message("NOT_FOUND", "mapping not found"))?;

    if m.mapping_type == "text" {
        if let Some(ref enc) = m.text_value {
            let plain = decrypt_value(value_key, enc)?;
            m.text_value = Some(String::from_utf8_lossy(&plain).into_owned());
        }
    }
    let enc_ph: Option<String> = conn
        .query_row(
            "SELECT proxy_placeholder FROM bucket_mappings WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .ok();
    m.proxy_placeholder = decrypt_placeholder(value_key, enc_ph)?;
    Ok(m)
}

fn normalize_env_label(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Ok(trimmed.to_uppercase());
    }
    Err(AppError::message(
        "VALIDATION_ERROR",
        "env name may only contain letters, numbers, and underscores",
    ))
}
