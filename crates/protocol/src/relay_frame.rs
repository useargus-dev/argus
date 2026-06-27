//! Metadata prefix on relay → bucket-proxy TCP connections (transparent capture).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub const HEADER_LEN: usize = 20;

/// Magic + version: `ARG\x01`.
pub const MAGIC: [u8; 4] = *b"ARG\x01";

/// Signed relay header: `MAGIC | pid: u32 BE | nonce: u64 BE | tag: u32` (first 4 bytes of HMAC-SHA256).
pub fn encode_signed(secret: &[u8; 32], captured_pid: u32, nonce: u64) -> [u8; HEADER_LEN] {
    let mut buf = [0u8; HEADER_LEN];
    buf[..4].copy_from_slice(&MAGIC);
    buf[4..8].copy_from_slice(&captured_pid.to_be_bytes());
    buf[8..16].copy_from_slice(&nonce.to_be_bytes());
    let tag = hmac_tag(secret, captured_pid, nonce);
    buf[16..20].copy_from_slice(&tag);
    buf
}

/// Returns `(captured_pid, nonce)` when `prefix` is a valid signed relay header.
pub fn decode_and_verify(secret: &[u8; 32], prefix: &[u8]) -> Option<(u32, u64)> {
    if prefix.len() < HEADER_LEN || prefix[..4] != MAGIC {
        return None;
    }
    let pid = u32::from_be_bytes(prefix[4..8].try_into().ok()?);
    let nonce = u64::from_be_bytes(prefix[8..16].try_into().ok()?);
    let tag = &prefix[16..20];
    let expected = hmac_tag(secret, pid, nonce);
    if bool::from(tag.ct_eq(&expected)) {
        Some((pid, nonce))
    } else {
        None
    }
}

fn hmac_tag(secret: &[u8; 32], pid: u32, nonce: u64) -> [u8; 4] {
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts 32-byte keys");
    mac.update(&MAGIC);
    mac.update(&pid.to_be_bytes());
    mac.update(&nonce.to_be_bytes());
    let result = mac.finalize().into_bytes();
    result[..4].try_into().expect("slice length")
}

/// First byte of relay output — used to peek before reading the rest of the header.
pub const fn first_byte() -> u8 {
    MAGIC[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_roundtrip() {
        let secret = [7u8; 32];
        let pid = 35_096u32;
        let nonce = 42u64;
        let hdr = encode_signed(&secret, pid, nonce);
        assert_eq!(decode_and_verify(&secret, &hdr), Some((pid, nonce)));
    }

    #[test]
    fn rejects_bad_tag() {
        let secret = [7u8; 32];
        let mut hdr = encode_signed(&secret, 100, 1);
        hdr[19] ^= 0xff;
        assert_eq!(decode_and_verify(&secret, &hdr), None);
    }

    #[test]
    fn rejects_short_or_wrong_magic() {
        let secret = [1u8; 32];
        assert_eq!(decode_and_verify(&secret, b"ARG"), None);
        assert_eq!(decode_and_verify(&secret, b"TLS\x01xxxx"), None);
    }
}
