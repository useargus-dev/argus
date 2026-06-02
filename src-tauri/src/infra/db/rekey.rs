use hex::encode as hex_encode;
use rusqlite::Connection;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::error::{AppError, AppResult};

fn reencrypt_optional(
    old_key: &[u8; 32],
    new_key: &[u8; 32],
    enc: Option<String>,
) -> AppResult<Option<String>> {
    match enc {
        None => Ok(None),
        Some(value) if value.is_empty() => Ok(Some(value)),
        Some(value) => {
            let plain = decrypt_value(old_key, &value)?;
            Ok(Some(encrypt_value(new_key, &plain)?))
        }
    }
}

pub fn reencrypt_all_values(
    conn: &Connection,
    old_value_key: &[u8; 32],
    new_value_key: &[u8; 32],
) -> AppResult<()> {
    {
        let enc: Option<String> = conn
            .query_row(
                "SELECT totp_secret FROM users WHERE id = 'local'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        if let Some(new_enc) = reencrypt_optional(old_value_key, new_value_key, enc)? {
            conn.execute(
                "UPDATE users SET totp_secret = ?1 WHERE id = 'local'",
                [new_enc],
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        }
    }

    {
        let mut stmt = conn
            .prepare("SELECT id, value FROM secrets")
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        for (id, enc) in rows {
            let plain = decrypt_value(old_value_key, &enc)?;
            let new_enc = encrypt_value(new_value_key, &plain)?;
            conn.execute(
                "UPDATE secrets SET value = ?1 WHERE id = ?2",
                rusqlite::params![new_enc, id],
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        }
    }

    {
        let mut stmt = conn
            .prepare("SELECT id, client_token_enc FROM app_buckets WHERE client_token_enc IS NOT NULL")
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        for (id, enc) in rows {
            let plain = decrypt_value(old_value_key, &enc)?;
            let new_enc = encrypt_value(new_value_key, &plain)?;
            conn.execute(
                "UPDATE app_buckets SET client_token_enc = ?1 WHERE id = ?2",
                rusqlite::params![new_enc, id],
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        }
    }

    {
        let mut stmt = conn
            .prepare(
                "SELECT id, text_value, proxy_placeholder FROM bucket_mappings
                 WHERE text_value IS NOT NULL OR proxy_placeholder IS NOT NULL",
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        let rows: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        for (id, text, proxy) in rows {
            let new_text = reencrypt_optional(old_value_key, new_value_key, text)?;
            let new_proxy = reencrypt_optional(old_value_key, new_value_key, proxy)?;
            conn.execute(
                "UPDATE bucket_mappings SET text_value = ?1, proxy_placeholder = ?2 WHERE id = ?3",
                rusqlite::params![new_text, new_proxy, id],
            )
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        }
    }

    Ok(())
}

/// After re-encrypting with a new value key, sync TOTP blob to OS keychain (sign-in reads from there).
pub fn sync_totp_keychain(conn: &Connection) -> AppResult<()> {
    use crate::infra::db::{meta, users};

    let user = users::get_user(conn)?;
    if user.totp_enabled {
        if let Some(ref enc) = user.totp_secret {
            meta::write_totp_secret_enc(enc)?;
        }
    }
    Ok(())
}

pub fn rekey_database(conn: &Connection, new_db_key: &[u8; 32]) -> AppResult<()> {
    let key_hex = hex_encode(new_db_key);
    let pragma = format!("PRAGMA rekey = \"x'{}'\";", key_hex);
    conn.execute_batch(&pragma)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    Ok(())
}
