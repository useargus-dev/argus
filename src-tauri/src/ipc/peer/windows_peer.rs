use std::os::windows::io::RawHandle;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId;
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;

use crate::error::{AppError, AppResult};

/// PID of the process that opened the other end of this named pipe.
pub fn client_pid_from_handle(handle: RawHandle) -> AppResult<u32> {
    let mut pid: u32 = 0;
    let ok = unsafe { GetNamedPipeClientProcessId(handle as HANDLE, &mut pid) };
    if ok == 0 {
        return Err(AppError::message(
            "PEER_RESOLVE",
            "GetNamedPipeClientProcessId failed (no client connected?)",
        ));
    }
    if pid == 0 {
        return Err(AppError::message("PEER_RESOLVE", "named pipe client pid is 0"));
    }
    assert_same_session(pid)?;
    Ok(pid)
}

/// Reject cross-session pipe clients (another Windows session on the same machine).
fn assert_same_session(client_pid: u32) -> AppResult<()> {
    let mut client_session: u32 = 0;
    let mut self_session: u32 = 0;
    let ok1 = unsafe { ProcessIdToSessionId(client_pid, &mut client_session) };
    let ok2 = unsafe { ProcessIdToSessionId(std::process::id(), &mut self_session) };
    if ok1 == 0 || ok2 == 0 {
        return Ok(());
    }
    if client_session != self_session {
        return Err(AppError::message(
            "PEER_DENIED",
            "pipe client is not in the current user session",
        ));
    }
    Ok(())
}
