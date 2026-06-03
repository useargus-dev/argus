use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;

use crate::error::{AppError, AppResult};

const MEMORY_KIB: u32 = 65536;
const TIME_COST: u32 = 3;
const PARALLELISM: u32 = 4;

pub fn argon2() -> AppResult<Argon2<'static>> {
    let params = Params::new(MEMORY_KIB, TIME_COST, PARALLELISM, Some(32))
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        params,
    ))
}

/// Hash password for storage (PHC string includes salt).
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, encoded_hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    Ok(argon2()?
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub struct DerivedKeys {
    pub db_key: [u8; 32],
    pub value_key: [u8; 32],
}

impl Drop for DerivedKeys {
    fn drop(&mut self) {
        self.db_key.zeroize();
        self.value_key.zeroize();
    }
}

/// Derive SQLCipher key and value encryption key from master password + stored hash salt.
pub fn derive_keys(password: &str, encoded_hash: &str) -> AppResult<DerivedKeys> {
    let parsed = PasswordHash::new(encoded_hash)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    let salt = parsed
        .salt
        .ok_or_else(|| AppError::message("CRYPTO_ERROR", "missing salt in password hash"))?
        .as_str()
        .as_bytes();

    let mut db_key = [0u8; 32];
    argon2()?
        .hash_password_into(password.as_bytes(), salt, &mut db_key)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;

    let hk = Hkdf::<Sha256>::new(None, &db_key);
    let mut value_key = [0u8; 32];
    hk.expand(b"argus-value-v1", &mut value_key)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;

    Ok(DerivedKeys { db_key, value_key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn derive_keys_deterministic_for_same_password() {
        let hash = hash_password("test-password-123").unwrap();
        let k1 = derive_keys("test-password-123", &hash).unwrap();
        let k2 = derive_keys("test-password-123", &hash).unwrap();
        assert_eq!(k1.db_key, k2.db_key);
        assert_eq!(k1.value_key, k2.value_key);
    }
}
