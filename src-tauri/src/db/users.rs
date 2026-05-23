use chrono::Utc;
use rusqlite::{params, Connection};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub second_factor_type: String,
    pub totp_enabled: bool,
    pub biometric_enrolled: bool,
}

pub fn count_users(conn: &Connection) -> AppResult<i64> {
    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn get_user(conn: &Connection) -> AppResult<UserRow> {
    conn.query_row(
        "SELECT email, username, avatar_url, password_hash, totp_secret,
                second_factor_type, totp_enabled, biometric_enrolled
         FROM users WHERE id = 'local'",
        [],
        |row| {
            Ok(UserRow {
                email: row.get(0)?,
                username: row.get(1)?,
                avatar_url: row.get(2)?,
                password_hash: row.get(3)?,
                totp_secret: row.get(4)?,
                second_factor_type: row.get(5)?,
                totp_enabled: row.get::<_, i64>(6)? != 0,
                biometric_enrolled: row.get::<_, i64>(7)? != 0,
            })
        },
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn get_profile(conn: &Connection) -> AppResult<UserProfile> {
    let user = get_user(conn)?;
    Ok(UserProfile {
        email: user.email,
        username: user.username,
        avatar_url: user.avatar_url,
    })
}

pub fn insert_user(
    conn: &Connection,
    email: &str,
    username: &str,
    avatar_url: Option<&str>,
    password_hash: &str,
    totp_secret_enc: Option<&str>,
    second_factor_type: &str,
    totp_enabled: bool,
    biometric_enrolled: bool,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO users (
            id, email, username, avatar_url, password_hash, totp_secret,
            second_factor_type, totp_enabled, biometric_enrolled, created_at, last_signed_in_at
        ) VALUES ('local', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
        ON CONFLICT(id) DO UPDATE SET
            email = excluded.email,
            username = excluded.username,
            avatar_url = excluded.avatar_url,
            password_hash = excluded.password_hash,
            totp_secret = excluded.totp_secret,
            second_factor_type = excluded.second_factor_type,
            totp_enabled = excluded.totp_enabled,
            biometric_enrolled = excluded.biometric_enrolled,
            last_signed_in_at = excluded.last_signed_in_at",
        params![
            email,
            username,
            avatar_url,
            password_hash,
            totp_secret_enc,
            second_factor_type,
            if totp_enabled { 1 } else { 0 },
            if biometric_enrolled { 1 } else { 0 },
            now,
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn update_last_signed_in(conn: &Connection) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE users SET last_signed_in_at = ?1 WHERE id = 'local'",
        [now],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn update_profile(
    conn: &Connection,
    avatar_url: Option<&str>,
) -> AppResult<UserProfile> {
    if let Some(url) = avatar_url {
        if !url.is_empty() && (!url.starts_with("https://") || url.contains(' ')) {
            return Err(AppError::message(
                "VALIDATION_ERROR",
                "avatar URL must be https",
            ));
        }
        conn.execute(
            "UPDATE users SET avatar_url = ?1 WHERE id = 'local'",
            [url],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    get_profile(conn)
}

pub fn find_by_identifier(conn: &Connection, identifier: &str) -> AppResult<UserRow> {
    let id = identifier.trim().to_lowercase();
    conn.query_row(
        "SELECT email, username, avatar_url, password_hash, totp_secret,
                second_factor_type, totp_enabled, biometric_enrolled
         FROM users
         WHERE lower(email) = ?1 OR lower(username) = ?1",
        [id],
        |row| {
            Ok(UserRow {
                email: row.get(0)?,
                username: row.get(1)?,
                avatar_url: row.get(2)?,
                password_hash: row.get(3)?,
                totp_secret: row.get(4)?,
                second_factor_type: row.get(5)?,
                totp_enabled: row.get::<_, i64>(6)? != 0,
                biometric_enrolled: row.get::<_, i64>(7)? != 0,
            })
        },
    )
    .map_err(|_| AppError::message("AUTH_FAILED", "invalid credentials"))
}
