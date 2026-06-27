//! Relay header + TLS peel path used by transparent proxy server.

use protocol::relay_frame;

#[test]
fn relay_header_precedes_tls_record() {
    let secret = [3u8; 32];
    let pid = 42_424u32;
    let hdr = relay_frame::encode_signed(&secret, pid, 7);
    let tls_record = [0x16u8; 5];
    let mut stream = Vec::new();
    stream.extend_from_slice(&hdr);
    stream.extend_from_slice(&tls_record);

    assert_eq!(stream.len(), relay_frame::HEADER_LEN + tls_record.len());
    assert_eq!(
        relay_frame::decode_and_verify(&secret, &stream[..relay_frame::HEADER_LEN]),
        Some((pid, 7))
    );
    assert_eq!(stream[relay_frame::HEADER_LEN], 0x16);
}

#[test]
fn relay_first_byte_routes_to_header_peel() {
    let secret = [1u8; 32];
    let hdr = relay_frame::encode_signed(&secret, 99, 1);
    assert_eq!(hdr[0], relay_frame::first_byte());
    assert_eq!(relay_frame::decode_and_verify(&secret, &hdr), Some((99, 1)));
}

#[test]
fn relay_rejects_bad_tag() {
    let secret = [2u8; 32];
    let mut hdr = relay_frame::encode_signed(&secret, 50, 3);
    hdr[19] ^= 0x55;
    assert_eq!(relay_frame::decode_and_verify(&secret, &hdr), None);
}
