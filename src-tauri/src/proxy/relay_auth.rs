//! Relay Tier 1 + Tier 2 verification for loopback transparent connections.

use protocol::capture_log;
use protocol::relay_frame;

use crate::infra::db::sandbox_sessions;
use crate::proxy::peer_relay::{diagnose_relay_peer, is_trusted_relay_peer};
use crate::sandbox::cache::{consume_relay_nonce, lookup_relay_secret_by_pid};

/// Verify relay header and return the authenticated captured PID.
pub fn verify_relay_header(
    conn: &rusqlite::Connection,
    bucket_id: &str,
    tcp_peer_pid: u32,
    hdr: &[u8; relay_frame::HEADER_LEN],
) -> Option<u32> {
    if !is_trusted_relay_peer(tcp_peer_pid) {
        capture_log::log(
            "relay-auth",
            format!(
                "tier1 peer trust failed tcp_peer_pid={tcp_peer_pid}: {}",
                diagnose_relay_peer(tcp_peer_pid)
            ),
        );
        return None;
    }

    let header_pid = match hdr[4..8].try_into().ok().map(u32::from_be_bytes) {
        Some(pid) => pid,
        None => {
            capture_log::log("relay-auth", "invalid relay header pid bytes");
            return None;
        }
    };

    let secret = match lookup_relay_secret_by_pid(conn, bucket_id, header_pid) {
        Ok(Some(s)) => s,
        Ok(None) => {
            capture_log::log(
                "relay-auth",
                format!(
                    "no relay secret for header_pid={header_pid} bucket={bucket_id} \
                     (session expired or pid not registered?)"
                ),
            );
            return None;
        }
        Err(e) => {
            capture_log::log(
                "relay-auth",
                format!("relay secret lookup error header_pid={header_pid}: {e}"),
            );
            return None;
        }
    };

    let (pid, nonce) = match relay_frame::decode_and_verify(&secret, hdr) {
        Some(v) => v,
        None => {
            capture_log::log(
                "relay-auth",
                format!("relay HMAC verify failed header_pid={header_pid} tcp_peer_pid={tcp_peer_pid}"),
            );
            return None;
        }
    };
    debug_assert_eq!(pid, header_pid);

    let session = match sandbox_sessions::lookup_active_session_by_pid(conn, pid) {
        Ok(Some(s)) => s,
        Ok(None) => {
            capture_log::log(
                "relay-auth",
                format!("no active sandbox session for captured pid={pid}"),
            );
            return None;
        }
        Err(e) => {
            capture_log::log(
                "relay-auth",
                format!("session lookup error pid={pid}: {e}"),
            );
            return None;
        }
    };

    if session.bucket_id != bucket_id {
        capture_log::log(
            "relay-auth",
            format!(
                "session bucket mismatch: session={} session_bucket={} proxy_bucket={bucket_id}",
                session.id, session.bucket_id
            ),
        );
        return None;
    }

    if !consume_relay_nonce(&session.id, nonce) {
        capture_log::log(
            "relay-auth",
            format!("relay nonce replay or missing session={} nonce={nonce}", session.id),
        );
        return None;
    }

    capture_log::log(
        "relay-auth",
        format!(
            "ok captured_pid={pid} tcp_peer_pid={tcp_peer_pid} session={}",
            session.id
        ),
    );
    Some(pid)
}
