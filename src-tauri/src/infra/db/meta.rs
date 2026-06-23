use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::infra::db::argus_dir;
use crate::error::{AppError, AppResult};
use crate::util::{fs as argus_fs, secure};

const MIGRATION_BASE: &str = include_str!("migrations/001_base.sql");
const MIGRATION_PROXY: &str = include_str!("migrations/002_proxy.sql");
const MIGRATION_SANDBOX: &str = include_str!("migrations/003_sandbox.sql");

#[derive(Debug, Serialize, Deserialize, Default)]
struct AccountMeta {
    has_account: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    second_factor_type: Option<String>,
}

pub fn meta_path() -> PathBuf {
    argus_dir().join("meta.json")
}

fn read_meta() -> AppResult<AccountMeta> {
    let path = meta_path();
    if !path.exists() {
        return Ok(AccountMeta::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| AppError::message("IO_ERROR", e.to_string()))
}

fn write_meta(meta: &AccountMeta) -> AppResult<()> {
    ensure_argus_dir()?;
    let raw = serde_json::to_string_pretty(meta)
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    let path = meta_path();
    fs::write(&path, raw).map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    argus_fs::harden_path(&path, false)?;
    Ok(())
}

pub fn read_has_account() -> AppResult<bool> {
    Ok(read_meta()?.has_account)
}

pub fn write_has_account() -> AppResult<()> {
    let mut meta = read_meta()?;
    meta.has_account = true;
    write_meta(&meta)
}

pub fn write_account_meta(second_factor_type: &str) -> AppResult<()> {
    let meta = AccountMeta {
        has_account: true,
        second_factor_type: Some(second_factor_type.to_string()),
    };
    write_meta(&meta)
}

pub fn update_second_factor_type(second_factor_type: &str) -> AppResult<()> {
    let mut meta = read_meta()?;
    meta.has_account = true;
    meta.second_factor_type = Some(second_factor_type.to_string());
    write_meta(&meta)
}

pub fn read_password_hash() -> AppResult<String> {
    secure::get_password_hash()
}

pub fn write_password_hash(hash: &str) -> AppResult<()> {
    secure::set_password_hash(hash)
}

pub fn read_second_factor_type() -> AppResult<String> {
    read_meta()?
        .second_factor_type
        .ok_or_else(|| AppError::message("NO_ACCOUNT", "account metadata missing"))
}

pub fn read_totp_secret_enc() -> AppResult<String> {
    secure::get_totp_enc()
}

pub fn write_totp_secret_enc(blob: &str) -> AppResult<()> {
    secure::set_totp_enc(blob)
}

pub fn write_recovery_escrow(recovery_hash: &str, escrow: &str) -> AppResult<()> {
    secure::set_recovery_hash(recovery_hash)?;
    secure::set_recovery_escrow(escrow)
}

pub fn read_recovery_hash() -> AppResult<String> {
    secure::get_recovery_hash()
}

pub fn read_recovery_escrow() -> AppResult<String> {
    secure::get_recovery_escrow()
}

pub fn clear_totp_secret_enc() -> AppResult<()> {
    secure::clear_totp_enc();
    Ok(())
}

pub fn ensure_argus_dir() -> AppResult<PathBuf> {
    let dir = argus_dir();
    fs::create_dir_all(&dir).map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    argus_fs::harden_path(&dir, true)?;
    Ok(dir)
}

pub fn db_path() -> PathBuf {
    argus_dir().join("argus.db")
}

pub fn reset_local_data() -> AppResult<bool> {
    let dir = argus_dir();
    secure::clear_all();
    if !dir.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&dir).map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    Ok(true)
}

pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    let version: i64 = conn
        .prepare("SELECT COALESCE(MAX(version), 0) FROM schema_migrations")
        .and_then(|mut stmt| stmt.query_row([], |row| row.get(0)))
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(MIGRATION_BASE)
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
            [Utc::now().to_rfc3339()],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }

    if version < 2 {
        conn.execute_batch(MIGRATION_PROXY)
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1)",
            [Utc::now().to_rfc3339()],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }

    if version < 3 {
        conn.execute_batch(MIGRATION_SANDBOX)
            .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (3, ?1)",
            [Utc::now().to_rfc3339()],
        )
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;
    }

    Ok(())
}
