use chrono::Utc;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::error::{AppError, AppResult};
use crate::messages;

pub const PROXY_PORT_MIN: u16 = 9000;
pub const PROXY_PORT_MAX: u16 = 9100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketMeta {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub access_ttl_minutes: i64,
    pub refresh_ttl_minutes: Option<i64>,
    pub session_ttl_minutes: i64,
    pub mapping_count: u32,
    pub active_grant_count: u32,
    pub proxy_enabled: bool,
    pub proxy_port: Option<u16>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketWithToken {
    #[serde(flatten)]
    pub meta: BucketMeta,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct BucketProxyRow {
    pub id: String,
    pub proxy_enabled: bool,
    pub proxy_port: Option<u16>,
}

const TOKEN_LEN: usize = 32;
const TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub fn generate_bucket_token() -> String {
    let mut rng = rand::thread_rng();
    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..TOKEN_CHARS.len());
            TOKEN_CHARS[idx] as char
        })
        .collect()
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn encrypt_token(value_key: &[u8; 32], token: &str) -> AppResult<String> {
    encrypt_value(value_key, token.as_bytes())
}

fn decrypt_token(value_key: &[u8; 32], enc: &str) -> AppResult<String> {
    let plain = decrypt_value(value_key, enc)?;
    String::from_utf8(plain).map_err(|_| AppError::message("DB_ERROR", "invalid token encoding"))
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<BucketMeta> {
    Ok(BucketMeta {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        is_active: row.get::<_, i64>(3)? != 0,
        access_ttl_minutes: row.get(4)?,
        refresh_ttl_minutes: row.get(5)?,
        session_ttl_minutes: row.get(6)?,
        mapping_count: row.get(7)?,
        active_grant_count: row.get(8)?,
        proxy_enabled: row.get::<_, i64>(9)? != 0,
        proxy_port: row.get::<_, Option<i64>>(10)?.map(|p| p as u16),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const LIST_SELECT: &str = r"
SELECT b.id, b.name, b.description, b.is_tray_active,
       b.access_ttl_minutes, b.refresh_ttl_minutes, b.session_ttl_minutes,
       (SELECT COUNT(*) FROM bucket_mappings m WHERE m.bucket_id = b.id),
       (SELECT COUNT(*) FROM client_grants g
        WHERE g.bucket_id = b.id AND g.expires_at > datetime('now')),
       b.proxy_enabled, b.proxy_port,
       b.created_at, b.updated_at
FROM app_buckets b
ORDER BY b.updated_at DESC
";

pub fn list_buckets(conn: &Connection) -> AppResult<Vec<BucketMeta>> {
    let mut stmt = conn
        .prepare(LIST_SELECT)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([], row_to_meta)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

const META_SELECT: &str = r"
SELECT b.id, b.name, b.description, b.is_tray_active,
       b.access_ttl_minutes, b.refresh_ttl_minutes, b.session_ttl_minutes,
       (SELECT COUNT(*) FROM bucket_mappings m WHERE m.bucket_id = b.id),
       (SELECT COUNT(*) FROM client_grants g
        WHERE g.bucket_id = b.id AND g.expires_at > datetime('now')),
       b.proxy_enabled, b.proxy_port,
       b.created_at, b.updated_at
FROM app_buckets b
WHERE b.id = ?1
";

pub fn get_bucket_meta(conn: &Connection, id: &str) -> AppResult<BucketMeta> {
    conn.query_row(META_SELECT, [id], row_to_meta).map_err(|_| {
        AppError::message("BUCKET_NOT_FOUND", messages::bucket_not_found(id))
    })
}

pub fn get_bucket_proxy_row(conn: &Connection, id: &str) -> AppResult<BucketProxyRow> {
    conn.query_row(
        "SELECT id, proxy_enabled, proxy_port FROM app_buckets WHERE id = ?1",
        [id],
        |r| {
            Ok(BucketProxyRow {
                id: r.get(0)?,
                proxy_enabled: r.get::<_, i64>(1)? != 0,
                proxy_port: r.get::<_, Option<i64>>(2)?.map(|p| p as u16),
            })
        },
    )
    .map_err(|_| {
        AppError::message("BUCKET_NOT_FOUND", messages::bucket_not_found(id))
    })
}

pub fn list_proxy_enabled_buckets(conn: &Connection) -> AppResult<Vec<(String, u16)>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, proxy_port FROM app_buckets WHERE proxy_enabled = 1 AND proxy_port IS NOT NULL",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([], |r| {
            let id: String = r.get(0)?;
            let port: i64 = r.get(1)?;
            Ok((id, port as u16))
        })
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

fn persist_token(
    conn: &Connection,
    value_key: &[u8; 32],
    id: &str,
    token: &str,
) -> AppResult<()> {
    let token_hash = hash_token(token);
    let token_enc = encrypt_token(value_key, token)?;
    conn.execute(
        "UPDATE app_buckets SET client_token_hash = ?2, client_token_enc = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, token_hash, token_enc, Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn allocate_proxy_port(conn: &Connection, bucket_id: &str) -> AppResult<u16> {
    let used: Vec<i64> = conn
        .prepare("SELECT proxy_port FROM app_buckets WHERE proxy_port IS NOT NULL AND id != ?1")
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
        .query_map([bucket_id], |r| r.get(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    for port in PROXY_PORT_MIN..=PROXY_PORT_MAX {
        if !used.contains(&(port as i64)) {
            return Ok(port);
        }
    }
    Err(AppError::message(
        "PROXY_PORT_EXHAUSTED",
        "no free proxy ports in range 9000-9100",
    ))
}

pub fn set_bucket_proxy_enabled(
    conn: &Connection,
    bucket_id: &str,
    enabled: bool,
) -> AppResult<BucketMeta> {
    let _ = get_bucket_meta(conn, bucket_id)?;
    let now = Utc::now().to_rfc3339();

    if enabled {
        let existing_port: Option<i64> = conn
            .query_row(
                "SELECT proxy_port FROM app_buckets WHERE id = ?1",
                [bucket_id],
                |r| r.get(0),
            )
            .unwrap_or(None);
        let port = match existing_port {
            Some(p) => p as u16,
            None => allocate_proxy_port(conn, bucket_id)?,
        };
        conn.execute(
            "UPDATE app_buckets SET proxy_enabled = 1, proxy_port = ?2, updated_at = ?3 WHERE id = ?1",
            params![bucket_id, port as i64, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    } else {
        conn.execute(
            "UPDATE app_buckets SET proxy_enabled = 0, updated_at = ?2 WHERE id = ?1",
            params![bucket_id, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }

    get_bucket_meta(conn, bucket_id)
}

pub fn create_bucket(
    conn: &Connection,
    value_key: &[u8; 32],
    name: &str,
    description: Option<&str>,
) -> AppResult<BucketWithToken> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::message("VALIDATION_ERROR", "name is required"));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let token = generate_bucket_token();
    let token_hash = hash_token(&token);
    let token_enc = encrypt_token(value_key, &token)?;

    conn.execute(
        "INSERT INTO app_buckets (id, name, description, client_token_hash, client_token_enc,
         access_ttl_minutes, refresh_ttl_minutes, session_ttl_minutes, is_tray_active,
         proxy_enabled, allowed_hosts, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 60, NULL, 480, 1, 0, '[]', ?6, ?6)",
        params![id, trimmed, description, token_hash, token_enc, now],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::message("VALIDATION_ERROR", "bucket name already exists")
        } else {
            AppError::message("DB_ERROR", e.to_string())
        }
    })?;

    let meta = get_bucket_meta(conn, &id)?;
    Ok(BucketWithToken { meta, token })
}

pub fn delete_bucket(conn: &Connection, id: &str) -> AppResult<()> {
    let n = conn
        .execute("DELETE FROM app_buckets WHERE id = ?1", [id])
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    if n == 0 {
        return Err(AppError::message(
            "BUCKET_NOT_FOUND",
            messages::bucket_not_found(id),
        ));
    }
    Ok(())
}

/// Toggle active state and always rotate the client token.
pub fn set_bucket_active(
    conn: &Connection,
    value_key: &[u8; 32],
    id: &str,
    active: bool,
) -> AppResult<BucketWithToken> {
    let _ = get_bucket_meta(conn, id)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE app_buckets SET is_tray_active = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, if active { 1 } else { 0 }, now],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let token = generate_bucket_token();
    persist_token(conn, value_key, id, &token)?;
    let meta = get_bucket_meta(conn, id)?;
    Ok(BucketWithToken { meta, token })
}

pub fn verify_client_token(
    conn: &Connection,
    bucket_id: &str,
    client_token: &str,
) -> AppResult<BucketMeta> {
    let meta = get_bucket_meta(conn, bucket_id)?;
    if !meta.is_active {
        return Err(AppError::message(
            "BUCKET_INACTIVE",
            messages::bucket_inactive(&meta.name),
        ));
    }
    let expected: String = conn
        .query_row(
            "SELECT client_token_hash FROM app_buckets WHERE id = ?1",
            [bucket_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            AppError::message("BUCKET_NOT_FOUND", messages::bucket_not_found(bucket_id))
        })?;
    let got = hash_token(client_token);
    if got != expected {
        return Err(AppError::message(
            "INVALID_TOKEN",
            messages::invalid_token(&meta.name),
        ));
    }
    Ok(meta)
}

pub fn verify_token_hash(conn: &Connection, client_token: &str) -> AppResult<String> {
    let hash = hash_token(client_token);
    let bucket_id: Option<String> = conn
        .query_row(
            "SELECT id FROM app_buckets WHERE client_token_hash = ?1",
            [&hash],
            |r| r.get(0),
        )
        .ok();
    bucket_id.ok_or_else(|| {
        AppError::message("INVALID_TOKEN", messages::invalid_token_generic())
    })
}

pub fn get_bucket_token(
    conn: &Connection,
    value_key: &[u8; 32],
    id: &str,
) -> AppResult<String> {
    let enc: Option<String> = conn
        .query_row(
            "SELECT client_token_enc FROM app_buckets WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| {
        AppError::message("BUCKET_NOT_FOUND", messages::bucket_not_found(id))
    })?;
    let enc = enc.ok_or_else(|| {
        AppError::message(
            "TOKEN_UNAVAILABLE",
            "token unavailable — toggle the bucket to regenerate",
        )
    })?;
    decrypt_token(value_key, &enc)
}
