//! Relay header + TLS peel path used by transparent proxy server.

use protocol::relay_frame;

#[test]
fn relay_header_precedes_tls_record() {
    let pid = 42_424u32;
    let hdr = relay_frame::encode(pid);
    let tls_record = [0x16u8; 5];
    let mut stream = Vec::new();
    stream.extend_from_slice(&hdr);
    stream.extend_from_slice(&tls_record);

    assert_eq!(stream.len(), relay_frame::HEADER_LEN + tls_record.len());
    assert_eq!(relay_frame::decode(&stream[..relay_frame::HEADER_LEN]), Some(pid));
    assert_eq!(stream[relay_frame::HEADER_LEN], 0x16);
}

#[test]
fn relay_first_byte_routes_to_header_peel() {
    let hdr = relay_frame::encode(99);
    assert_eq!(hdr[0], relay_frame::first_byte());
    assert_eq!(relay_frame::decode(&hdr), Some(99));
}
