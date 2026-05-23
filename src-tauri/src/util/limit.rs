use chrono::{Duration, Utc};

use crate::error::{AppError, AppResult};
use crate::state::AppStateInner;

const MAX_FAILURES: u32 = 10;
const LOCKOUT_MINUTES: i64 = 15;

pub fn check_lockout(inner: &AppStateInner) -> AppResult<()> {
    if let Some(until) = inner.auth_lockout_until {
        if until > Utc::now() {
            let mins = (until - Utc::now()).num_minutes().max(1);
            return Err(AppError::message(
                "AUTH_LOCKED",
                format!("Too many attempts. Try again in {mins} minutes."),
            ));
        }
    }
    Ok(())
}

pub fn record_failure(inner: &mut AppStateInner) {
    inner.auth_failures = inner.auth_failures.saturating_add(1);
    if inner.auth_failures >= MAX_FAILURES {
        inner.auth_lockout_until = Some(Utc::now() + Duration::minutes(LOCKOUT_MINUTES));
        inner.auth_failures = 0;
    }
}

pub fn clear_failures(inner: &mut AppStateInner) {
    inner.auth_failures = 0;
    inner.auth_lockout_until = None;
}
