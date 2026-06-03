use std::sync::OnceLock;

/// Reads a stable machine identifier from the OS (cached after first read).
/// Windows: MachineGuid from registry (Win32 API, no subprocess).
/// Linux: /etc/machine-id.
/// macOS: IOPlatformUUID via ioreg.

static MACHINE_ID: OnceLock<String> = OnceLock::new();

pub fn read_machine_id() -> String {
    MACHINE_ID
        .get_or_init(|| {
            platform_machine_id().unwrap_or_else(|| "unknown-machine".to_string())
        })
        .clone()
}

#[cfg(target_os = "windows")]
fn platform_machine_id() -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    unsafe {
        let subkey = wide(r"SOFTWARE\Microsoft\Cryptography");
        let value_name = wide("MachineGuid");
        let mut hkey = std::ptr::null_mut();

        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != ERROR_SUCCESS
        {
            return None;
        }

        let mut data_type = 0u32;
        let mut data_len = 0u32;
        let len_status = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_len,
        );
        if len_status != ERROR_SUCCESS || data_type != REG_SZ || data_len < 2 {
            RegCloseKey(hkey);
            return None;
        }

        let wchar_count = (data_len as usize) / 2;
        let mut buffer = vec![0u16; wchar_count];
        let query_status = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            buffer.as_mut_ptr().cast(),
            &mut data_len,
        );
        RegCloseKey(hkey);

        if query_status != ERROR_SUCCESS {
            return None;
        }

        let value = String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .trim()
            .to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
}

#[cfg(target_os = "linux")]
fn platform_machine_id() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn platform_machine_id() -> Option<String> {
    use std::process::Command;
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains("IOPlatformUUID") {
            if let Some(start) = line.find('"') {
                let rest = &line[start + 1..];
                if let Some(end) = rest.rfind('"') {
                    let uuid = &rest[..end];
                    if !uuid.is_empty() {
                        return Some(uuid.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn platform_machine_id() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_machine_id_is_cached_and_non_empty() {
        let first = read_machine_id();
        let second = read_machine_id();
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }
}
