//! OS-verified IPC client fingerprint.
//!
//! Derives identity from the pipe/socket peer process using kernel-guaranteed
//! PID, then inspects exe, cwd, command line, UID, machine ID, and git remote.
//! The resulting SHA-256 fingerprint is used for grant lookup — client JSON is
//! never trusted for identity.

pub mod machine_id;
mod resolve;

#[cfg(unix)]
mod unix_peer;
#[cfg(windows)]
mod windows_peer;

use crate::error::AppResult;

pub use resolve::VerifiedClient;

/// Resolve the process attached to an IPC connection (platform-specific).
#[cfg(windows)]
pub fn from_connected_stream<S>(stream: &S, fallback_cwd: Option<&str>) -> AppResult<VerifiedClient>
where
    S: std::os::windows::io::AsRawHandle,
{
    let pid = windows_peer::client_pid_from_handle(stream.as_raw_handle())?;
    VerifiedClient::from_pid(pid, fallback_cwd)
}

#[cfg(unix)]
pub fn from_connected_stream<S>(stream: &S, fallback_cwd: Option<&str>) -> AppResult<VerifiedClient>
where
    S: std::os::unix::io::AsRawFd,
{
    let pid = unix_peer::client_pid_from_fd(stream.as_raw_fd())?;
    VerifiedClient::from_pid(pid, fallback_cwd)
}

#[cfg(not(any(windows, unix)))]
pub fn from_connected_stream<S>(_stream: &S, _fallback_cwd: Option<&str>) -> AppResult<VerifiedClient> {
    Err(AppError::message(
        "PEER_RESOLVE",
        "IPC peer verification is not supported on this platform",
    ))
}
