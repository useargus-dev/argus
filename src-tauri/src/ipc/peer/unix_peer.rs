use std::os::unix::io::{BorrowedFd, RawFd};

use nix::sys::socket::getsockopt;

use crate::error::{AppError, AppResult};

/// PID (and on Linux, UID) of the process connected to this Unix domain socket.
pub fn client_pid_from_fd(fd: RawFd) -> AppResult<u32> {
    let pid = peer_pid(fd)?;
    assert_same_user(fd)?;
    if pid == 0 {
        return Err(AppError::message("PEER_RESOLVE", "invalid peer pid"));
    }
    Ok(pid)
}

#[cfg(target_os = "linux")]
fn peer_pid(fd: RawFd) -> AppResult<u32> {
    use nix::sys::socket::sockopt::PeerCredentials;
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let cred = getsockopt(&borrowed, PeerCredentials)
        .map_err(|e| AppError::message("PEER_RESOLVE", format!("SO_PEERCRED failed: {e}")))?;
    Ok(cred.pid() as u32)
}

#[cfg(target_os = "macos")]
fn peer_pid(fd: RawFd) -> AppResult<u32> {
    use nix::sys::socket::sockopt::LocalPeerPid;
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let pid: i32 = getsockopt(&borrowed, LocalPeerPid)
        .map_err(|e| AppError::message("PEER_RESOLVE", format!("LOCAL_PEERPID failed: {e}")))?;
    Ok(pid as u32)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn peer_pid(_fd: RawFd) -> AppResult<u32> {
    Err(AppError::message(
        "PEER_RESOLVE",
        "Unix peer PID is only supported on Linux and macOS",
    ))
}

#[cfg(target_os = "linux")]
fn assert_same_user(fd: RawFd) -> AppResult<()> {
    use nix::sys::socket::sockopt::PeerCredentials;
    use nix::unistd::getuid;
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let cred = getsockopt(&borrowed, PeerCredentials)
        .map_err(|e| AppError::message("PEER_RESOLVE", format!("SO_PEERCRED failed: {e}")))?;
    let me = getuid();
    if cred.uid() != me.as_raw() {
        return Err(AppError::message(
            "PEER_DENIED",
            "socket peer is a different user",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn assert_same_user(_fd: RawFd) -> AppResult<()> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn assert_same_user(_fd: RawFd) -> AppResult<()> {
    Ok(())
}
