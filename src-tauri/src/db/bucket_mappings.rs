use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::db::buckets;
use crate::db::secrets;
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
        created_at: row.get(8)?,
    })
}

const LIST_SQL: &str = r"
SELECT m.id, m.bucket_id, m.env_label, m.mapping_type, m.secret_id, s.name, s.secret_type, m.text_value, m.created_at
FROM bucket_mappings m
LEFT JOIN secrets s ON s.id = m.secret_id AND s.is_archived = 0
WHERE m.bucket_id = ?1
ORDER BY m.env_label COLLATE NOCASE
";

pub fn list_mappings(conn: &Connection, bucket_id: &str, value_key: &[u8; 32]) -> AppResult<Vec<BucketMapping>> {
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
    value_key: &[u8; 32],
) -> AppResult<BucketMapping> {
    let _bucket = buckets::get_bucket_meta(conn, bucket_id)?;
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
            "UPDATE bucket_mappings SET mapping_type = ?2, secret_id = ?3, text_value = ?4 WHERE id = ?1",
            params![id, mapping_type, secret_id, encrypted_text],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id, value_key)
    } else {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO bucket_mappings (id, bucket_id, env_label, mapping_type, secret_id, text_value, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, bucket_id, label, mapping_type, secret_id, encrypted_text, now],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        get_mapping(conn, &id, value_key)
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

fn get_mapping(conn: &Connection, id: &str, value_key: &[u8; 32]) -> AppResult<BucketMapping> {
    let mut m = conn.query_row(
        r"
SELECT m.id, m.bucket_id, m.env_label, m.mapping_type, m.secret_id, s.name, s.secret_type, m.text_value, m.created_at
FROM bucket_mappings m
LEFT JOIN secrets s ON s.id = m.secret_id
WHERE m.id = ?1
",
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
