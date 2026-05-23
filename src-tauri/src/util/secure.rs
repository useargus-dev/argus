use keyring::Entry;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "com.useargus.dev";
const KEY_PASSWORD_HASH: &str = "password_hash";
const KEY_TOTP_ENC: &str = "totp_secret_enc";

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

pub fn clear_all() {
    if let Ok(e) = entry(KEY_PASSWORD_HASH) {
        let _ = e.delete_credential();
    }
    if let Ok(e) = entry(KEY_TOTP_ENC) {
        let _ = e.delete_credential();
    }
}
