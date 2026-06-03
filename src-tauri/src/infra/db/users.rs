use chrono::Utc;
use rusqlite::{params, Connection};

use crate::error::{AppError, AppResult};

pub const PLACEHOLDER_EMAIL: &str = "unset@local.argus";

/// Email for registration when the user skips it (editable later in Settings).
pub fn resolve_register_email(email: &str) -> String {
    if email.is_empty() {
        PLACEHOLDER_EMAIL.to_string()
    } else {
        email.to_lowercase()
    }
}

/// Username slug when omitted at registration (editable later in Settings).
pub fn default_register_username(username: &str, first_name: &str, last_name: &str) -> String {
    if !username.is_empty() {
        return username.to_string();
    }
    let mut slug = String::new();
    for part in [first_name.trim(), last_name.trim()] {
        if part.is_empty() {
            continue;
        }
        if !slug.is_empty() {
            slug.push('-');
        }
        for c in part.chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if c == ' ' || c == '-' || c == '_' {
                slug.push('-');
            }
        }
    }
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug = slug.trim_matches('-').to_string();
    if slug.len() >= 2 {
        slug
    } else {
        "master".to_string()
    }
}

pub fn is_placeholder_email(email: &str) -> bool {
    email.eq_ignore_ascii_case(PLACEHOLDER_EMAIL)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub email: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub email: String,
    pub username: String,
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
        "SELECT email, username, password_hash, totp_secret,
                second_factor_type, totp_enabled, biometric_enrolled
         FROM users WHERE id = 'local'",
        [],
        |row| {
            Ok(UserRow {
                email: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                totp_secret: row.get(3)?,
                second_factor_type: row.get(4)?,
                totp_enabled: row.get::<_, i64>(5)? != 0,
                biometric_enrolled: row.get::<_, i64>(6)? != 0,
            })
        },
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn get_profile(conn: &Connection) -> AppResult<UserProfile> {
    conn.query_row(
        "SELECT email, username, first_name, last_name FROM users WHERE id = 'local'",
        [],
        |row| {
            Ok(UserProfile {
                email: row.get(0)?,
                username: row.get(1)?,
                first_name: row.get(2)?,
                last_name: row.get(3)?,
            })
        },
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

pub fn insert_user(
    conn: &Connection,
    email: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    password_hash: &str,
    totp_secret_enc: Option<&str>,
    second_factor_type: &str,
    totp_enabled: bool,
    biometric_enrolled: bool,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO users (
            id, email, username, first_name, last_name, avatar_url, password_hash, totp_secret,
            second_factor_type, totp_enabled, biometric_enrolled, created_at, last_signed_in_at
        ) VALUES ('local', ?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
        ON CONFLICT(id) DO UPDATE SET
            email = excluded.email,
            username = excluded.username,
            first_name = excluded.first_name,
            last_name = excluded.last_name,
            password_hash = excluded.password_hash,
            totp_secret = excluded.totp_secret,
            second_factor_type = excluded.second_factor_type,
            totp_enabled = excluded.totp_enabled,
            biometric_enrolled = excluded.biometric_enrolled,
            last_signed_in_at = excluded.last_signed_in_at",
        params![
            email,
            username,
            first_name,
            last_name,
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
    email: Option<&str>,
    username: Option<&str>,
    first_name: Option<&str>,
    last_name: Option<&str>,
) -> AppResult<UserProfile> {
    if let Some(email) = email {
        let email = email.trim().to_lowercase();
        if email.is_empty() || !email.contains('@') {
            return Err(AppError::message("VALIDATION_ERROR", "invalid email"));
        }
        conn.execute(
            "UPDATE users SET email = ?1 WHERE id = 'local'",
            [email],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    if let Some(username) = username {
        let username = username.trim();
        if username.len() < 2 {
            return Err(AppError::message("VALIDATION_ERROR", "username too short"));
        }
        conn.execute(
            "UPDATE users SET username = ?1 WHERE id = 'local'",
            [username],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    if let Some(first_name) = first_name {
        conn.execute(
            "UPDATE users SET first_name = ?1 WHERE id = 'local'",
            [first_name.trim()],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    if let Some(last_name) = last_name {
        conn.execute(
            "UPDATE users SET last_name = ?1 WHERE id = 'local'",
            [last_name.trim()],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }
    get_profile(conn)
}

pub fn set_totp_enrolled(conn: &Connection, totp_secret_enc: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE users SET totp_secret = ?1, totp_enabled = 1 WHERE id = 'local'",
        [totp_secret_enc],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn set_biometric_enrolled(conn: &Connection, enrolled: bool) -> AppResult<()> {
    conn.execute(
        "UPDATE users SET biometric_enrolled = ?1 WHERE id = 'local'",
        [if enrolled { 1 } else { 0 }],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn update_password_hash(conn: &Connection, password_hash: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = 'local'",
        [password_hash],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

/// Replace the sole second factor — clears all others.
pub fn replace_second_factor(
    conn: &Connection,
    second_factor_type: &str,
    totp_secret_enc: Option<&str>,
    biometric_enrolled: bool,
) -> AppResult<()> {
    let typ = second_factor_type.to_lowercase();
    if typ != "totp" && typ != "biometric" {
        return Err(AppError::message("VALIDATION_ERROR", "invalid second factor"));
    }
    if typ == "totp" && totp_secret_enc.is_none() {
        return Err(AppError::message("VALIDATION_ERROR", "TOTP secret required"));
    }
    conn.execute(
        "UPDATE users SET
            second_factor_type = ?1,
            totp_enabled = ?2,
            totp_secret = ?3,
            biometric_enrolled = ?4
         WHERE id = 'local'",
        params![
            typ,
            if typ == "totp" { 1 } else { 0 },
            totp_secret_enc,
            if typ == "biometric" && biometric_enrolled { 1 } else { 0 },
        ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn set_active_second_factor(conn: &Connection, second_factor_type: &str) -> AppResult<()> {
    let user = get_user(conn)?;
    let typ = second_factor_type.to_lowercase();
    if typ == "totp" && !user.totp_enabled {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "register TOTP before enabling it",
        ));
    }
    if typ == "biometric" && !user.biometric_enrolled {
        return Err(AppError::message(
            "VALIDATION_ERROR",
            "register biometric before enabling it",
        ));
    }
    if typ != "totp" && typ != "biometric" {
        return Err(AppError::message("VALIDATION_ERROR", "invalid second factor"));
    }
    conn.execute(
        "UPDATE users SET second_factor_type = ?1 WHERE id = 'local'",
        [typ],
    )
    .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}

pub fn find_by_identifier(conn: &Connection, identifier: &str) -> AppResult<UserRow> {
    let id = identifier.trim().to_lowercase();
    conn.query_row(
        "SELECT email, username, password_hash, totp_secret,
                second_factor_type, totp_enabled, biometric_enrolled
         FROM users
         WHERE lower(email) = ?1 OR lower(username) = ?1",
        [id],
        |row| {
            Ok(UserRow {
                email: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                totp_secret: row.get(3)?,
                second_factor_type: row.get(4)?,
                totp_enabled: row.get::<_, i64>(5)? != 0,
                biometric_enrolled: row.get::<_, i64>(6)? != 0,
            })
        },
    )
    .map_err(|_| AppError::message("AUTH_FAILED", "invalid credentials"))
}
