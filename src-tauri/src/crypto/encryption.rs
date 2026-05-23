use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;

use crate::error::{AppError, AppResult};

const NONCE_LEN: usize = 12;

pub fn encrypt_value(value_key: &[u8; 32], plaintext: &[u8]) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(value_key)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(blob))
}

pub fn decrypt_value(value_key: &[u8; 32], encoded: &str) -> AppResult<Vec<u8>> {
    let blob = STANDARD
        .decode(encoded)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    if blob.len() <= NONCE_LEN {
        return Err(AppError::message("CRYPTO_ERROR", "ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(value_key)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::message("CRYPTO_ERROR", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let key = [7u8; 32];
        let plain = b"super-secret-api-key";
        let enc = encrypt_value(&key, plain).unwrap();
        let dec = decrypt_value(&key, &enc).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let key = [1u8; 32];
        let wrong = [2u8; 32];
        let enc = encrypt_value(&key, b"data").unwrap();
        assert!(decrypt_value(&wrong, &enc).is_err());
    }
}
