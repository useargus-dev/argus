use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::Rng;

use crate::crypto::encryption::{decrypt_value, encrypt_value};
use crate::crypto::kdf::argon2;
use crate::error::{AppError, AppResult};

/// Crockford-style alphabet (no 0/O, 1/I/L).
const RECOVERY_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

pub fn normalize_recovery_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

pub fn is_valid_recovery_code(raw: &str) -> bool {
    let code = normalize_recovery_code(raw);
    code.len() == 8
        && code
            .bytes()
            .all(|b| RECOVERY_ALPHABET.contains(&b))
}

pub fn generate_recovery_code() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..RECOVERY_ALPHABET.len());
            RECOVERY_ALPHABET[idx] as char
        })
        .collect()
}

pub fn hash_recovery_code(code: &str) -> AppResult<String> {
    let code = normalize_recovery_code(code);
    if !is_valid_recovery_code(&code) {
        return Err(AppError::message("VALIDATION_ERROR", "invalid recovery code"));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2()?
        .hash_password(code.as_bytes(), &salt)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(hash.to_string())
}

pub fn verify_recovery_code(code: &str, encoded_hash: &str) -> AppResult<bool> {
    let code = normalize_recovery_code(code);
    if !is_valid_recovery_code(&code) {
        return Ok(false);
    }
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(argon2()?
        .verify_password(code.as_bytes(), &parsed)
        .is_ok())
}

pub fn derive_recovery_key(code: &str, encoded_hash: &str) -> AppResult<[u8; 32]> {
    let code = normalize_recovery_code(code);
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    let salt = parsed
        .salt
        .ok_or_else(|| AppError::message("CRYPTO_ERROR", "missing salt in recovery hash"))?
        .as_str()
        .as_bytes();

    let mut key = [0u8; 32];
    argon2()?
        .hash_password_into(code.as_bytes(), salt, &mut key)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(key)
}

pub fn wrap_session_keys(
    recovery_key: &[u8; 32],
    db_key: &[u8; 32],
    value_key: &[u8; 32],
) -> AppResult<String> {
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(db_key);
    buf[32..].copy_from_slice(value_key);
    encrypt_value(recovery_key, &buf)
}

pub fn unwrap_session_keys(recovery_key: &[u8; 32], encoded: &str) -> AppResult<([u8; 32], [u8; 32])> {
    let plain = decrypt_value(recovery_key, encoded)?;
    if plain.len() != 64 {
        return Err(AppError::message(
            "CRYPTO_ERROR",
            "invalid recovery escrow payload",
        ));
    }
    let mut db_key = [0u8; 32];
    let mut value_key = [0u8; 32];
    db_key.copy_from_slice(&plain[..32]);
    value_key.copy_from_slice(&plain[32..]);
    Ok((db_key, value_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_code_round_trip() {
        let code = generate_recovery_code();
        assert_eq!(code.len(), 8);
        let hash = hash_recovery_code(&code).unwrap();
        assert!(verify_recovery_code(&code, &hash).unwrap());
        assert!(!verify_recovery_code("ZZZZZZZZ", &hash).unwrap());
    }

    #[test]
    fn normalize_strips_hyphen() {
        assert_eq!(normalize_recovery_code("abcd-efgh"), "ABCDEFGH");
    }
}
