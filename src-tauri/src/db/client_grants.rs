use chrono::{Duration, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::db::buckets;
use crate::db::settings;
use crate::error::{AppError, AppResult};

pub struct ActiveGrant {
    pub id: String,
}

pub fn find_active_grant(
    conn: &Connection,
    bucket_id: &str,
    fingerprint: &str,
    token_hash: &str,
) -> AppResult<Option<ActiveGrant>> {
    let row: Result<String, _> = conn.query_row(
        "SELECT id FROM client_grants
         WHERE bucket_id = ?1 AND fingerprint = ?2 AND token_hash = ?3
           AND expires_at > datetime('now')",
        params![bucket_id, fingerprint, token_hash],
        |r| r.get(0),
    );
    match row {
        Ok(id) => Ok(Some(ActiveGrant { id })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::message("DB_ERROR", e.to_string())),
    }
}

pub fn touch_grant(conn: &Connection, grant_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE client_grants SET last_seen_at = ?2 WHERE id = ?1",
        params![grant_id, Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn insert_grant(
    conn: &Connection,
    bucket_id: &str,
    fingerprint: &str,
    token: &str,
    ttl_minutes: i64,
    client_label: Option<&str>,
) -> AppResult<String> {
    let token_hash = buckets::hash_token(token);
    let now = Utc::now();
    let expires = now + Duration::minutes(ttl_minutes.max(1));
    let now_s = now.to_rfc3339();
    let exp_s = expires.to_rfc3339();

    if let Some(existing) = find_active_grant(conn, bucket_id, fingerprint, &token_hash)? {
        conn.execute(
            "UPDATE client_grants SET expires_at = ?2, last_seen_at = ?3,
             client_label = COALESCE(?4, client_label) WHERE id = ?1",
            params![existing.id, exp_s, now_s, client_label],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        return Ok(existing.id);
    }

    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO client_grants (id, bucket_id, fingerprint, token_hash,
         client_label, granted_at, expires_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(bucket_id, fingerprint, token_hash) DO UPDATE SET
           client_label = COALESCE(excluded.client_label, client_label),
           granted_at = excluded.granted_at,
           expires_at = excluded.expires_at,
           last_seen_at = excluded.last_seen_at",
        params![id, bucket_id, fingerprint, token_hash, client_label, now_s, exp_s, now_s],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let grant_id: String = conn
        .query_row(
            "SELECT id FROM client_grants WHERE bucket_id = ?1 AND fingerprint = ?2 AND token_hash = ?3",
            params![bucket_id, fingerprint, token_hash],
            |r| r.get(0),
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(grant_id)
}

/// A grant row for the UI approvals list.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantRow {
    pub id: String,
    pub bucket_id: String,
    pub bucket_name: String,
    pub fingerprint: String,
    pub client_label: Option<String>,
    pub granted_at: String,
    pub expires_at: String,
    pub last_seen_at: Option<String>,
    pub is_active: bool,
}

pub fn list_grants(conn: &Connection) -> AppResult<Vec<GrantRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.bucket_id, COALESCE(b.name, '(deleted)'), g.fingerprint,
                    g.client_label, g.granted_at, g.expires_at, g.last_seen_at
             FROM client_grants g
             LEFT JOIN app_buckets b ON b.id = g.bucket_id
             ORDER BY g.granted_at DESC",
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let rows = stmt
        .query_map([], |row| {
            let expires_at: String = row.get(6)?;
            let is_active = expires_at.as_str() > Utc::now().to_rfc3339().as_str();
            Ok(GrantRow {
                id: row.get(0)?,
                bucket_id: row.get(1)?,
                bucket_name: row.get(2)?,
                fingerprint: row.get(3)?,
                client_label: row.get(4)?,
                granted_at: row.get(5)?,
                expires_at,
                last_seen_at: row.get(7)?,
                is_active,
            })
        })
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| AppError::message("DB_ERROR", e.to_string()))?);
    }
    Ok(result)
}

pub fn revoke_grant(conn: &Connection, grant_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM client_grants WHERE id = ?1", params![grant_id])
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn access_ttl_minutes(conn: &Connection, bucket_access_ttl: i64) -> AppResult<i64> {
    if bucket_access_ttl > 0 {
        return Ok(bucket_access_ttl);
    }
    let raw = settings::get_or_default(conn, "default_access_ttl_minutes", "60")?;
    let parsed: i64 = raw
        .parse()
        .map_err(|_| AppError::message("DB_ERROR", "invalid default_access_ttl_minutes"))?;
    Ok(parsed.max(1))
}
