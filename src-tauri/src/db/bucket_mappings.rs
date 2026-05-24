use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::buckets;
use crate::db::secrets;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketMapping {
    pub id: String,
    pub bucket_id: String,
    pub env_label: String,
    pub secret_id: String,
    pub secret_name: String,
    pub secret_type: String,
    pub created_at: String,
}

fn row_to_mapping(row: &rusqlite::Row<'_>) -> rusqlite::Result<BucketMapping> {
    Ok(BucketMapping {
        id: row.get(0)?,
        bucket_id: row.get(1)?,
        env_label: row.get(2)?,
        secret_id: row.get(3)?,
        secret_name: row.get(4)?,
        secret_type: row.get(5)?,
        created_at: row.get(6)?,
    })
}

const LIST_SQL: &str = r"
SELECT m.id, m.bucket_id, m.env_label, m.secret_id, s.name, s.secret_type, m.created_at
FROM bucket_mappings m
INNER JOIN secrets s ON s.id = m.secret_id
WHERE m.bucket_id = ?1 AND s.is_archived = 0
ORDER BY m.env_label COLLATE NOCASE
";

pub fn list_mappings(conn: &Connection, bucket_id: &str) -> AppResult<Vec<BucketMapping>> {
    let mut stmt = conn
        .prepare(LIST_SQL)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    let rows = stmt
        .query_map([bucket_id], row_to_mapping)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn upsert_mapping(
    conn: &Connection,
    bucket_id: &str,
    env_label: &str,
    secret_id: &str,
) -> AppResult<BucketMapping> {
    let _bucket = buckets::get_bucket_meta(conn, bucket_id)?;
    let label = normalize_env_label(env_label)?;
    if label.is_empty() {
        return Err(AppError::message("VALIDATION_ERROR", "env name is required"));
    }

    secrets::get_secret_meta(conn, secret_id)?;

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM bucket_mappings WHERE bucket_id = ?1 AND env_label = ?2",
            params![bucket_id, label],
            |r| r.get(0),
        )
        .ok();

    let now = Utc::now().to_rfc3339();
    if let Some(id) = existing {
        conn.execute(
            "UPDATE bucket_mappings SET secret_id = ?2 WHERE id = ?1",
            params![id, secret_id],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id)
    } else {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO bucket_mappings (id, bucket_id, env_label, secret_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, bucket_id, label, secret_id, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id)
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

fn get_mapping(conn: &Connection, id: &str) -> AppResult<BucketMapping> {
    conn.query_row(
        r"
SELECT m.id, m.bucket_id, m.env_label, m.secret_id, s.name, s.secret_type, m.created_at
FROM bucket_mappings m
INNER JOIN secrets s ON s.id = m.secret_id
WHERE m.id = ?1
",
        [id],
        row_to_mapping,
    )
    .map_err(|_| AppError::message("NOT_FOUND", "mapping not found"))
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
