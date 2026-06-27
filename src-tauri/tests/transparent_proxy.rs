//! Integration-style tests for transparent sandbox gate and protocol sniff routing.

mod common;

use argus_lib::db::meta::run_migrations;
use argus_lib::db::sandbox_sessions;
use argus_lib::proxy::server::{route_incoming_first_byte, IncomingRoute};
use argus_lib::proxy::transparent::{evaluate_transparent_gate, TransparentGateResult};
use argus_lib::util::process_identity::process_boot_id;
use protocol::test_fixtures::minimal_client_hello_with_sni;
use rusqlite::Connection;

#[test]
fn connect_proxy_first_byte_unchanged() {
    assert_eq!(route_incoming_first_byte(b'C'), IncomingRoute::MaybeHttp);
    assert_ne!(route_incoming_first_byte(b'C'), IncomingRoute::TransparentTls);
}

#[test]
fn mock_tls_client_session_gate_allows_rewrite_path() {
    let conn = mem_conn();
    let bucket_id = common::seed_bucket(&conn, r#"["api.example.com"]"#);
    common::seed_grant(&conn, &bucket_id, "grant-1");
    let pid = std::process::id();
    let session = sandbox_sessions::create_session(
        &conn,
        &bucket_id,
        "grant-1",
        "fp-test",
        Some("curl https://api.example.com"),
        60,
    )
    .unwrap();
    let boot_id = process_boot_id(pid).expect("live pid");
    sandbox_sessions::register_pids(&conn, &session.id, &[(pid, boot_id)]).unwrap();
    let vk = [0u8; 32];
    let prefix = minimal_client_hello_with_sni("api.example.com");
    match evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(pid), &prefix) {
        TransparentGateResult::Ok(ok) => {
            assert_eq!(ok.session_id, session.id);
            assert_eq!(ok.host, "api.example.com");
        }
        TransparentGateResult::Deny { reason, .. } => {
            panic!("expected session gate to allow, got {reason}")
        }
    }
}

#[test]
fn unregistered_pid_denied() {
    let conn = mem_conn();
    let bucket_id = common::seed_bucket(&conn, r#"["api.example.com"]"#);
    let vk = [0u8; 32];
    let prefix = minimal_client_hello_with_sni("api.example.com");
    assert!(matches!(
        evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(99999), &prefix),
        TransparentGateResult::Deny { .. }
    ));
}

#[test]
fn revoked_session_denied() {
    let conn = mem_conn();
    let bucket_id = common::seed_bucket(&conn, r#"["api.example.com"]"#);
    common::seed_grant(&conn, &bucket_id, "grant-1");
    let pid = std::process::id();
    let session = sandbox_sessions::create_session(
        &conn,
        &bucket_id,
        "grant-1",
        "fp",
        None,
        60,
    )
    .unwrap();
    let boot_id = process_boot_id(pid).expect("live pid");
    sandbox_sessions::register_pids(&conn, &session.id, &[(pid, boot_id)]).unwrap();
    sandbox_sessions::revoke_session(&conn, &session.id).unwrap();
    let vk = [0u8; 32];
    let prefix = minimal_client_hello_with_sni("api.example.com");
    assert!(matches!(
        evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(pid), &prefix),
        TransparentGateResult::Deny { .. }
    ));
}

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}
