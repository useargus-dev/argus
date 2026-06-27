//! OS process identity helpers (boot ID, liveness).

use crate::error::{AppError, AppResult};

/// Stable process identity for the lifetime of a process (survives PID reuse).
pub fn process_boot_id(pid: u32) -> AppResult<String> {
    if pid == 0 {
        return Err(AppError::message("PROCESS_ID", "invalid pid"));
    }
    read_boot_id(pid).ok_or_else(|| {
        AppError::message(
            "PROCESS_ID",
            format!("could not read process identity for pid {pid}"),
        )
    })
}

pub fn process_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    probe_alive(pid)
}

#[cfg(target_os = "linux")]
fn read_boot_id(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Field 22 (1-indexed) after comm in parens is starttime.
    let after_comm = stat.rsplit(')').next()?.trim();
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    // Field 22 (1-indexed) in proc(5) is starttime; first token after comm is field 3.
    fields.get(19).map(|s| s.to_string())
}

#[cfg(target_os = "linux")]
fn probe_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(windows)]
fn read_boot_id(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut created = windows_sys::Win32::Foundation::FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit = created;
        let mut kernel = created;
        let mut user = created;
        let ok = GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user);
        CloseHandle(handle);
        if ok == 0 {
            return None;
        }
        let t = ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64;
        Some(format!("{t}"))
    }
}

#[cfg(windows)]
fn probe_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        CloseHandle(handle);
        true
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn read_boot_id(_pid: u32) -> Option<String> {
    None
}

#[cfg(not(any(target_os = "linux", windows)))]
fn probe_alive(_pid: u32) -> bool {
    false
}
