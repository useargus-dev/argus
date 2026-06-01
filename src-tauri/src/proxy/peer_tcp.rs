use crate::error::{AppError, AppResult};

#[cfg(unix)]
pub fn peer_pid_from_stream<S>(stream: &S) -> AppResult<u32>
where
    S: std::os::unix::io::AsRawFd,
{
    use std::os::fd::RawFd;
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();

    #[cfg(target_os = "linux")]
    {
        use nix::sys::socket::getsockopt;
        use nix::sys::socket::sockopt::PeerCredentials;
        let cred = getsockopt(fd, PeerCredentials)
            .map_err(|e| AppError::message("PEER_RESOLVE", e.to_string()))?;
        let pid = cred.pid();
        if pid <= 0 {
            return Err(AppError::message("PEER_RESOLVE", "invalid peer pid"));
        }
        return Ok(pid as u32);
    }

    #[cfg(target_os = "macos")]
    {
        use std::os::raw::c_int;
        const LOCAL_PEERPID: c_int = 0x002;
        let mut pid: u32 = 0;
        let r = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                LOCAL_PEERPID,
                &mut pid as *mut _ as *mut _,
                &mut (std::mem::size_of::<u32>() as u32) as *mut _,
            )
        };
        if r != 0 || pid == 0 {
            return Err(AppError::message("PEER_RESOLVE", "could not read peer pid"));
        }
        return Ok(pid);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = fd;
        Err(AppError::message(
            "PEER_RESOLVE",
            "TCP peer pid not supported on this unix platform",
        ))
    }
}

#[cfg(windows)]
pub fn peer_pid_from_stream<S>(_stream: &S) -> AppResult<u32> {
    Err(AppError::message(
        "PEER_RESOLVE",
        "TCP peer pid not available on Windows; using token-only grant check",
    ))
}

#[cfg(not(any(unix, windows)))]
pub fn peer_pid_from_stream<S>(_stream: &S) -> AppResult<u32> {
    Err(AppError::message("PEER_RESOLVE", "unsupported platform"))
}
