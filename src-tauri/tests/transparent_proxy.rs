//! Integration-style tests for transparent sandbox gate and protocol sniff routing.

use argus_lib::proxy::server::{route_incoming_first_byte, IncomingRoute};
use argus_lib::proxy::transparent::{evaluate_transparent_gate, TransparentGateResult};
use argus_lib::db::meta::run_migrations;
use argus_lib::db::sandbox_sessions;
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

fn seed_bucket(conn: &Connection) -> String {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc,
         access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts,
         created_at, updated_at)
         VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, '[\"api.example.com\"]', ?2, ?2)",
        params![id, now],
    )
    .unwrap();
    id
}

fn tls_client_hello(host: &str) -> Vec<u8> {
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
    record
}

#[test]
fn connect_proxy_first_byte_unchanged() {
    assert_eq!(route_incoming_first_byte(b'C'), IncomingRoute::MaybeHttp);
    assert_ne!(route_incoming_first_byte(b'C'), IncomingRoute::TransparentTls);
}

#[test]
fn mock_tls_client_session_gate_allows_rewrite_path() {
    let conn = mem_conn();
    let bucket_id = seed_bucket(&conn);
    let session = sandbox_sessions::create_session(
        &conn,
        &bucket_id,
        "grant-1",
        "fp-test",
        Some("curl https://api.example.com"),
        60,
    )
    .unwrap();
    sandbox_sessions::register_pids(&conn, &session.id, &[12345]).unwrap();
    let vk = [0u8; 32];
    let prefix = tls_client_hello("api.example.com");
    match evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(12345), &prefix) {
        TransparentGateResult::Ok(ok) => {
            assert_eq!(ok.session_id, session.id);
            assert_eq!(ok.host, "api.example.com");
        }
        TransparentGateResult::Deny { .. } => panic!("expected session gate to allow"),
    }
}

#[test]
fn unregistered_pid_denied() {
    let conn = mem_conn();
    let bucket_id = seed_bucket(&conn);
    let vk = [0u8; 32];
    let prefix = tls_client_hello("api.example.com");
    assert!(matches!(
        evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(99999), &prefix),
        TransparentGateResult::Deny { .. }
    ));
}

#[test]
fn revoked_session_denied() {
    let conn = mem_conn();
    let bucket_id = seed_bucket(&conn);
    let session = sandbox_sessions::create_session(
        &conn,
        &bucket_id,
        "grant-1",
        "fp",
        None,
        60,
    )
    .unwrap();
    sandbox_sessions::register_pids(&conn, &session.id, &[12345]).unwrap();
    sandbox_sessions::revoke_session(&conn, &session.id).unwrap();
    let vk = [0u8; 32];
    let prefix = tls_client_hello("api.example.com");
    assert!(matches!(
        evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(12345), &prefix),
        TransparentGateResult::Deny { .. }
    ));
}
