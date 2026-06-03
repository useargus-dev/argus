use base64::Engine;
use rusqlite::Connection;

use crate::infra::db::buckets;
use crate::infra::db::client_grants;
use crate::error::{AppError, AppResult};
use crate::ipc::VerifiedClient;

pub fn parse_proxy_token(headers: &[(String, String)]) -> Option<String> {
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("proxy-authorization") {
            if let Some(b64) = v.strip_prefix("Basic ").or_else(|| v.strip_prefix("basic ")) {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
                    if let Ok(s) = String::from_utf8(decoded) {
                        let token = s.split(':').next().unwrap_or(&s).trim();
                        if !token.is_empty() {
                            return Some(token.to_string());
                        }
                    }
                }
            }
            let token = v.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub struct ProxyAuth {
    pub bucket_id: String,
    pub token_hash: String,
}

pub fn authenticate_proxy_headers(
    conn: &Connection,
    headers: &[(String, String)],
) -> AppResult<ProxyAuth> {
    let token = parse_proxy_token(headers).ok_or_else(|| {
        AppError::message("PROXY_AUTH", "missing Proxy-Authorization")
    })?;
    let bucket_id = buckets::verify_token_hash(conn, &token)?;
    Ok(ProxyAuth {
        bucket_id,
        token_hash: buckets::hash_token(&token),
    })
}

/// Requires an OS-verified peer fingerprint matching an active IPC grant.
pub fn verify_grant(
    conn: &Connection,
    auth: &ProxyAuth,
    peer: Option<&VerifiedClient>,
) -> AppResult<bool> {
    let Some(p) = peer else {
        return Ok(false);
    };
    Ok(
        client_grants::find_active_grant(
            conn,
            &auth.bucket_id,
            &p.fingerprint,
            &auth.token_hash,
        )?
        .is_some(),
    )
}
