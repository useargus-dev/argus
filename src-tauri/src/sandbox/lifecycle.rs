//! Sandbox session revoke + cache invalidation helpers.

use rusqlite::Connection;

use crate::error::AppResult;
use crate::infra::db::sandbox_sessions;
use crate::sandbox::cache::{invalidate_revoked_sessions, invalidate_sessions_for_bucket};
use crate::sandbox::service::revoke_all_sessions_for_bucket;

pub fn purge_revoked_sessions(revoked: &[(String, Vec<u32>)]) {
    invalidate_revoked_sessions(revoked);
}

pub fn revoke_sessions_for_grant(conn: &Connection, grant_id: &str) -> AppResult<()> {
    let revoked = sandbox_sessions::revoke_sessions_by_grant_id(conn, grant_id)?;
    purge_revoked_sessions(&revoked);
    Ok(())
}

pub fn revoke_and_purge_bucket_sessions(conn: &Connection, bucket_id: &str) -> AppResult<()> {
    let revoked = revoke_all_sessions_for_bucket(conn, bucket_id)?;
    purge_revoked_sessions(&revoked);
    invalidate_sessions_for_bucket(bucket_id);
    Ok(())
}
