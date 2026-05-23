use totp_rs::{Algorithm, Secret, TOTP};

use crate::error::{AppError, AppResult};

const ISSUER: &str = "Argus";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_uri: String,
}

pub fn generate_totp_secret(account_label: &str) -> AppResult<TotpSetup> {
    let secret = Secret::generate_secret();
    let secret_b32 = secret.to_encoded().to_string();
    let totp = build_totp(&secret_b32, account_label)?;
    Ok(TotpSetup {
        secret: secret_b32,
        otpauth_uri: totp.get_url(),
    })
}

pub fn verify_totp_code(secret_b32: &str, code: &str) -> AppResult<bool> {
    let totp = build_totp(secret_b32, "user")?;
    let trimmed = code.trim();
    if trimmed.len() != 6 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }
    Ok(totp.check_current(trimmed).unwrap_or(false))
}

pub fn encrypt_totp_secret(value_key: &[u8; 32], secret_b32: &str) -> AppResult<String> {
    crate::crypto::encryption::encrypt_value(value_key, secret_b32.as_bytes())
}

pub fn decrypt_totp_secret(value_key: &[u8; 32], encrypted: &str) -> AppResult<String> {
    let bytes = crate::crypto::encryption::decrypt_value(value_key, encrypted)?;
    String::from_utf8(bytes).map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))
}

fn build_totp(secret_b32: &str, account: &str) -> AppResult<TOTP> {
    let secret = Secret::Encoded(secret_b32.to_string());
    let bytes = secret
        .to_bytes()
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;

    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some(ISSUER.to_string()),
        account.to_string(),
    )
    .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totp_generate_and_verify() {
        let setup = generate_totp_secret("test@argus").unwrap();
        let totp = build_totp(&setup.secret, "test@argus").unwrap();
        let code = totp.generate_current().unwrap();
        assert!(verify_totp_code(&setup.secret, &code).unwrap());
    }
}
