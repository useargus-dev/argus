//! Shared TLS test fixtures for transparent proxy gate tests.

/// Minimal TLS ClientHello record with SNI extension for `host`.
pub fn minimal_client_hello_with_sni(host: &str) -> Vec<u8> {
    let mut hello = vec![0x03, 0x03];
    hello.extend_from_slice(&[0u8; 32]);
    hello.push(0);
    hello.extend_from_slice(&[0, 2, 0x00, 0x2f]);
    hello.push(1);
    hello.push(0);
    let host_bytes = host.as_bytes();
    let sni_list_len = 3 + host_bytes.len();
    let ext_len = 2 + sni_list_len;
    let mut exts = Vec::new();
    exts.extend_from_slice(&[0x00, 0x00]);
    exts.extend_from_slice(&(ext_len as u16).to_be_bytes());
    exts.extend_from_slice(&(sni_list_len as u16).to_be_bytes());
    exts.push(0);
    exts.extend_from_slice(&(host_bytes.len() as u16).to_be_bytes());
    exts.extend_from_slice(host_bytes);
    hello.extend_from_slice(&(exts.len() as u16).to_be_bytes());
    hello.extend_from_slice(&exts);
    let hello_len = hello.len();
    let mut body = vec![0x01];
    body.extend_from_slice(&(hello_len as u32).to_be_bytes()[1..]);
    body.extend_from_slice(&hello);
    let record_len = body.len();
    let mut record = vec![0x16, 0x03, 0x01];
    record.extend_from_slice(&(record_len as u16).to_be_bytes());
    record.extend_from_slice(&body);
    record
}
