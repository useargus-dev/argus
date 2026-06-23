//! Metadata prefix on relay → bucket-proxy TCP connections (transparent capture).

pub const HEADER_LEN: usize = 8;

/// Magic + version: `ARG\x01`.
pub const MAGIC: [u8; 4] = *b"ARG\x01";

/// Prefix written once before the first TLS byte from intercept relay to `127.0.0.1:{port}`.
pub fn encode(captured_pid: u32) -> [u8; HEADER_LEN] {
    let mut buf = [0u8; HEADER_LEN];
    buf[..4].copy_from_slice(&MAGIC);
    buf[4..8].copy_from_slice(&captured_pid.to_be_bytes());
    buf
}

/// Returns captured PID when `prefix` is a full relay header.
pub fn decode(prefix: &[u8]) -> Option<u32> {
    if prefix.len() < HEADER_LEN || prefix[..4] != MAGIC {
        return None;
    }
    Some(u32::from_be_bytes(
        prefix[4..8].try_into().expect("slice length"),
    ))
}

/// First byte of [`encode`] output — used to peek before reading the rest of the header.
pub const fn first_byte() -> u8 {
    MAGIC[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let pid = 35_096u32;
        let hdr = encode(pid);
        assert_eq!(decode(&hdr), Some(pid));
    }

    #[test]
    fn rejects_short_or_wrong_magic() {
        assert_eq!(decode(b"ARG"), None);
        assert_eq!(decode(b"TLS\x01xxxx"), None);
    }
}
