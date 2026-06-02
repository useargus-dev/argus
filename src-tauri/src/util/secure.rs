use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.useargus.dev";
const KEY_PASSWORD_HASH: &str = "password_hash";
const KEY_TOTP_ENC: &str = "totp_secret_enc";
const KEY_RECOVERY_HASH: &str = "recovery_code_hash";
const KEY_RECOVERY_ESCROW: &str = "recovery_keys_enc";

fn entry(name: &str) -> AppResult<Entry> {
    Entry::new(SERVICE, name).map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn set_password_hash(hash: &str) -> AppResult<()> {
    entry(KEY_PASSWORD_HASH)?
        .set_password(hash)
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn get_password_hash() -> AppResult<String> {
    entry(KEY_PASSWORD_HASH)?
        .get_password()
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn set_totp_enc(blob: &str) -> AppResult<()> {
    entry(KEY_TOTP_ENC)?
        .set_password(blob)
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn get_totp_enc() -> AppResult<String> {
    entry(KEY_TOTP_ENC)?
        .get_password()
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn set_recovery_hash(hash: &str) -> AppResult<()> {
    entry(KEY_RECOVERY_HASH)?
        .set_password(hash)
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn get_recovery_hash() -> AppResult<String> {
    entry(KEY_RECOVERY_HASH)?
        .get_password()
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn set_recovery_escrow(blob: &str) -> AppResult<()> {
    entry(KEY_RECOVERY_ESCROW)?
        .set_password(blob)
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn get_recovery_escrow() -> AppResult<String> {
    entry(KEY_RECOVERY_ESCROW)?
        .get_password()
        .map_err(|e| AppError::message("KEYRING_ERROR", e.to_string()))
}

pub fn clear_totp_enc() {
    if let Ok(e) = entry(KEY_TOTP_ENC) {
        let _ = e.delete_credential();
    }
}

pub fn clear_all() {
    if let Ok(e) = entry(KEY_PASSWORD_HASH) {
        let _ = e.delete_credential();
    }
    if let Ok(e) = entry(KEY_TOTP_ENC) {
        let _ = e.delete_credential();
    }
    if let Ok(e) = entry(KEY_RECOVERY_HASH) {
        let _ = e.delete_credential();
    }
    if let Ok(e) = entry(KEY_RECOVERY_ESCROW) {
        let _ = e.delete_credential();
    }
}
