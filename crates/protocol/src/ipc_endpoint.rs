//! Shared IPC endpoint naming (Windows session-scoped named pipe).

/// Legacy global pipe name (pre-0.3.0). Kept for reference only.
pub const LEGACY_WINDOWS_PIPE: &str = r"\\.\pipe\argus";

/// Windows session id for the current process, or `0` if lookup fails.
#[cfg(windows)]
pub fn windows_session_id() -> u32 {
    use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;

    let mut session_id: u32 = 0;
    let ok = unsafe { ProcessIdToSessionId(std::process::id(), &mut session_id) };
    if ok != 0 {
        session_id
    } else {
        0
    }
}

#[cfg(not(windows))]
pub fn windows_session_id() -> u32 {
    0
}

/// Local IPC endpoint: session-scoped named pipe on Windows, not used on Unix.
#[cfg(windows)]
pub fn ipc_pipe_name() -> String {
    let session = windows_session_id();
    if session == 0 {
        LEGACY_WINDOWS_PIPE.to_string()
    } else {
        format!(r"\\.\pipe\argus-{session}")
    }
}

#[cfg(not(windows))]
pub fn ipc_pipe_name() -> String {
    LEGACY_WINDOWS_PIPE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_name_contains_argus_prefix() {
        let name = ipc_pipe_name();
        assert!(name.contains("argus"));
    }

    #[cfg(windows)]
    #[test]
    fn pipe_name_is_session_scoped_when_session_known() {
        if windows_session_id() != 0 {
            assert!(ipc_pipe_name().starts_with(r"\\.\pipe\argus-"));
        }
    }
}
