//! Shared DB fixtures for integration tests.

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

pub fn seed_bucket(conn: &Connection, allowed_hosts: &str) -> String {
    let id = Uuid::new_v4().to_string();
    let map_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc,
         access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts,
         created_at, updated_at)
         VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, ?2, ?3, ?3)",
        params![id, allowed_hosts, now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO bucket_mappings (id, bucket_id, env_label, mapping_type, text_value,
         proxy_enabled, allowed_hosts, created_at)
         VALUES (?1, ?2, 'API', 'text', '', 1, ?3, ?4)",
        params![map_id, id, allowed_hosts, now],
    )
    .unwrap();
    id
}

pub fn seed_grant(conn: &Connection, bucket_id: &str, grant_id: &str) {
    let now = Utc::now().to_rfc3339();
    let exp = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    conn.execute(
        "INSERT INTO client_grants (id, bucket_id, fingerprint, token_hash, granted_at, expires_at, last_seen_at)
         VALUES (?1, ?2, 'fp-test', 'hash', ?3, ?4, ?3)",
        params![grant_id, bucket_id, now, exp],
    )
    .unwrap();
}
