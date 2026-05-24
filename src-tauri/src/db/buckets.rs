use chrono::Utc;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::error::{AppError, AppResult};

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

fn hash_token(token: &str) -> String {
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
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const LIST_SELECT: &str = r"
SELECT b.id, b.name, b.description, b.is_tray_active,
       b.access_ttl_minutes, b.refresh_ttl_minutes, b.session_ttl_minutes,
       (SELECT COUNT(*) FROM bucket_mappings m WHERE m.bucket_id = b.id),
       (SELECT COUNT(*) FROM client_grants g
        WHERE g.bucket_id = b.id AND g.expires_at > datetime('now')),
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
       b.created_at, b.updated_at
FROM app_buckets b
WHERE b.id = ?1
";

pub fn get_bucket_meta(conn: &Connection, id: &str) -> AppResult<BucketMeta> {
    conn.query_row(META_SELECT, [id], row_to_meta)
        .map_err(|_| AppError::message("NOT_FOUND", "bucket not found"))
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
         created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 60, NULL, 480, 1, ?6, ?6)",
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
        return Err(AppError::message("NOT_FOUND", "bucket not found"));
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
        .map_err(|_| AppError::message("NOT_FOUND", "bucket not found"))?;
    let enc = enc.ok_or_else(|| {
        AppError::message(
            "TOKEN_UNAVAILABLE",
            "token unavailable — toggle the bucket to regenerate",
        )
    })?;
    decrypt_token(value_key, &enc)
}
