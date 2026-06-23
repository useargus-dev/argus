//! Minimal TLS ClientHello SNI parser (no full TLS stack).

use crate::error::{AppError, AppResult};

const TLS_HANDSHAKE: u8 = 0x16;
const HANDSHAKE_CLIENT_HELLO: u8 = 0x01;
const EXT_SERVER_NAME: u16 = 0x0000;
const NAME_HOST_NAME: u8 = 0;

/// Extract hostname from a TLS ClientHello record (first record only).
pub fn sni_from_client_hello(record: &[u8]) -> AppResult<Option<String>> {
    if record.len() < 5 || record[0] != TLS_HANDSHAKE {
        return Ok(None);
    }
    let record_len = u16_be(&record[3..5]) as usize;
    if record.len() < 5 + record_len {
        return Err(AppError::message("TLS_PARSE", "truncated TLS record"));
    }
    let body = &record[5..5 + record_len];
    if body.is_empty() || body[0] != HANDSHAKE_CLIENT_HELLO {
        return Ok(None);
    }
    parse_client_hello(&body[1..])
}

fn u24(b: &[u8]) -> u32 {
    ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32)
}

fn parse_client_hello(body: &[u8]) -> AppResult<Option<String>> {
    if body.len() < 2 {
        return Ok(None);
    }
    let hello_len = u24(&body[0..3]) as usize;
    if body.len() < 3 + hello_len {
        return Ok(None);
    }
    let hello = &body[3..3 + hello_len];
    // version(2) + random(32)
    if hello.len() < 34 {
        return Ok(None);
    }
    let mut i = 34;
    if i >= hello.len() {
        return Ok(None);
    }
    let sess_len = hello[i] as usize;
    i += 1 + sess_len;
    if i >= hello.len() {
        return Ok(None);
    }
    let cipher_len = u16_be(&hello[i..i + 2]) as usize;
    i += 2 + cipher_len;
    if i >= hello.len() {
        return Ok(None);
    }
    let comp_len = hello[i] as usize;
    i += 1 + comp_len;
    if i + 2 > hello.len() {
        return Ok(None);
    }
    let ext_total = u16_be(&hello[i..i + 2]) as usize;
    i += 2;
    if i + ext_total > hello.len() {
        return Ok(None);
    }
    let exts = &hello[i..i + ext_total];
    parse_extensions(exts)
}

fn u16_be(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}

fn parse_extensions(exts: &[u8]) -> AppResult<Option<String>> {
    let mut i = 0;
    while i + 4 <= exts.len() {
        let ext_type = u16_be(&exts[i..i + 2]);
        let ext_len = u16_be(&exts[i + 2..i + 4]) as usize;
        i += 4;
        if i + ext_len > exts.len() {
            break;
        }
        let ext_data = &exts[i..i + ext_len];
        if ext_type == EXT_SERVER_NAME {
            if let Some(host) = parse_sni_list(ext_data)? {
                return Ok(Some(host));
            }
        }
        i += ext_len;
    }
    Ok(None)
}

fn parse_sni_list(data: &[u8]) -> AppResult<Option<String>> {
    if data.len() < 2 {
        return Ok(None);
    }
    let list_len = u16_be(&data[0..2]) as usize;
    if data.len() < 2 + list_len {
        return Ok(None);
    }
    let mut i = 2;
    while i + 3 <= 2 + list_len {
        let name_type = data[i];
        let name_len = u16_be(&data[i + 1..i + 3]) as usize;
        i += 3;
        if i + name_len > 2 + list_len {
            break;
        }
        if name_type == NAME_HOST_NAME {
            let host = std::str::from_utf8(&data[i..i + name_len])
                .map_err(|e| AppError::message("TLS_PARSE", e.to_string()))?;
            return Ok(Some(host.to_string()));
        }
        i += name_len;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sni_from_standard_record() {
        let host = "api.anthropic.com";
        let mut hello = vec![0x03, 0x03];
        hello.extend_from_slice(&[0u8; 32]);
        hello.push(0);
        hello.extend_from_slice(&[0, 2, 0x00, 0x2f]);
        hello.push(1);
        hello.push(0);
        let host_bytes = host.as_bytes();
        let sni_list_len = 3 + host_bytes.len();
        let ext_len = 2 + sni_list_len;
        let mut exts = vec![0x00, 0x00];
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
        assert_eq!(
            sni_from_client_hello(&record).unwrap().as_deref(),
            Some(host)
        );
    }

    #[test]
    fn rejects_non_tls() {
        assert!(sni_from_client_hello(b"GET /").unwrap().is_none());
    }
}
