pub mod audit;
pub mod bucket_mappings;
pub mod buckets;
pub mod client_grants;
pub mod hosts;
pub mod ipc_env;
pub mod meta;
pub mod rekey;
pub mod secrets;
pub mod settings;
pub mod users;

pub use meta::{ensure_argus_dir, reset_local_data};

use std::path::PathBuf;
use std::sync::Mutex;

use hex::encode as hex_encode;
use rusqlite::Connection;
use secrecy::{ExposeSecret, SecretBox};

use crate::infra::db::meta::{db_path, run_migrations};
use crate::util::fs as argus_fs;
use crate::error::{AppError, AppResult};

pub type DbPool = Mutex<Connection>;

pub fn argus_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("ARGUS_DATA_DIR") {
        return PathBuf::from(override_dir);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".argus")
}

pub fn open_db(db_key: &[u8; 32]) -> AppResult<DbPool> {
    ensure_argus_dir()?;
    let path = db_path();
    let exists = path.exists();

    let conn = Connection::open(&path).map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    let key_hex = hex_encode(db_key);
    let pragma = format!("PRAGMA key = \"x'{}'\";", key_hex);
    conn.execute_batch(&pragma)
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))?;

    // Verify key
    conn.query_row("SELECT count(*) FROM sqlite_schema", [], |_| Ok(()))
        .map_err(|_| AppError::message("DB_ERROR", "invalid database key"))?;

    if !exists {
        argus_fs::harden_path(&path, false)?;
    }

    run_migrations(&conn)?;
    Ok(Mutex::new(conn))
}

pub fn wrap_db_key(db_key: [u8; 32]) -> SecretBox<[u8; 32]> {
    SecretBox::new(Box::new(db_key))
}

pub fn expose_db_key(key: &SecretBox<[u8; 32]>) -> [u8; 32] {
    *key.expose_secret()
}
