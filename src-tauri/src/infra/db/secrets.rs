use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretMeta {
    pub id: String,
    pub name: String,
    pub secret_type: String,
    pub organization: Option<String>,
    pub environment: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub expires_at: Option<String>,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDetail {
    #[serde(flatten)]
    pub meta: SecretMeta,
    pub value: serde_json::Value,
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecretMeta> {
    let tags_raw: Option<String> = row.get(5)?;
    let tags = tags_raw
        .as_deref()
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or_default();
    Ok(SecretMeta {
        id: row.get(0)?,
        name: row.get(1)?,
        secret_type: row.get(2)?,
        organization: row.get(3)?,
        environment: row.get(4)?,
        description: row.get(6)?,
        tags,
        expires_at: row.get(7)?,
        is_archived: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const META_SELECT: &str = "SELECT id, name, secret_type, organization, environment, tags,
       description, expires_at, is_archived, created_at, updated_at FROM secrets";

pub fn search_secrets(conn: &Connection, query: Option<&str>) -> AppResult<Vec<SecretMeta>> {
    if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
        let pattern = format!("%{}%", q.trim());
        let mut stmt = conn
            .prepare(&format!(
                "{META_SELECT} WHERE is_archived = 0 AND (name LIKE ?1 OR IFNULL(description,'') LIKE ?1)
             ORDER BY updated_at DESC"
            ))
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let rows = stmt
            .query_map([pattern], row_to_meta)
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        return rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()));
    }

    let mut stmt = conn
        .prepare(&format!(
            "{META_SELECT} WHERE is_archived = 0 ORDER BY updated_at DESC"
        ))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([], row_to_meta)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn get_secret_meta(conn: &Connection, id: &str) -> AppResult<SecretMeta> {
    conn.query_row(
        &format!("{META_SELECT} WHERE id = ?1"),
        [id],
        row_to_meta,
    )
    .map_err(|e| AppError::message("NOT_FOUND", e.to_string()))
}

pub fn get_secret_detail(
    conn: &Connection,
    id: &str,
    value_key: &[u8; 32],
) -> AppResult<SecretDetail> {
    let meta = get_secret_meta(conn, id)?;
    let enc: String = conn
        .query_row("SELECT value FROM secrets WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|e| AppError::message("NOT_FOUND", e.to_string()))?;
    let plain = decrypt_value(value_key, &enc)?;
    let value: serde_json::Value = serde_json::from_slice(&plain)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(SecretDetail { meta, value })
}

pub fn create_secret(
    conn: &Connection,
    value_key: &[u8; 32],
    name: &str,
    secret_type: &str,
    organization: Option<&str>,
    environment: Option<&str>,
    description: Option<&str>,
    tags: &[String],
    expires_at: Option<&str>,
    value: &serde_json::Value,
) -> AppResult<SecretMeta> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(tags)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let plain = serde_json::to_vec(value)
        .map_err(|e| AppError::message("VALIDATION_ERROR", e.to_string()))?;
    let enc = encrypt_value(value_key, &plain)?;

    conn.execute(
        "INSERT INTO secrets (id, name, secret_type, organization, environment, description,
         tags, value, expires_at, is_archived, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?10)",
        params![
            id,
            name.trim(),
            secret_type,
            organization,
            environment,
            description,
            tags_json,
            enc,
            expires_at,
            now,
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    get_secret_meta(conn, &id)
}

pub fn update_secret(
    conn: &Connection,
    value_key: &[u8; 32],
    id: &str,
    name: &str,
    secret_type: &str,
    organization: Option<&str>,
    environment: Option<&str>,
    description: Option<&str>,
    tags: &[String],
    expires_at: Option<&str>,
    value: &serde_json::Value,
) -> AppResult<SecretMeta> {
    let _ = get_secret_meta(conn, id)?;
    let now = Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(tags)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let plain = serde_json::to_vec(value)
        .map_err(|e| AppError::message("VALIDATION_ERROR", e.to_string()))?;
    let enc = encrypt_value(value_key, &plain)?;

    conn.execute(
        "UPDATE secrets SET name = ?2, secret_type = ?3, organization = ?4, environment = ?5,
         description = ?6, tags = ?7, value = ?8, expires_at = ?9, updated_at = ?10
         WHERE id = ?1",
        params![
            id,
            name.trim(),
            secret_type,
            organization,
            environment,
            description,
            tags_json,
            enc,
            expires_at,
            now,
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    get_secret_meta(conn, &id)
}

pub fn delete_secret(conn: &Connection, id: &str) -> AppResult<()> {
    let n = conn
        .execute("DELETE FROM secrets WHERE id = ?1", [id])
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    if n == 0 {
        return Err(AppError::message("NOT_FOUND", "secret not found"));
    }
    Ok(())
}
