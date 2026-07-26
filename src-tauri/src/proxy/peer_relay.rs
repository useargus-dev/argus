//! Tier 1 attestation for loopback relay connections to the bucket proxy.
//!
//! The intercept relay in `argus-cli` opens TCP to `127.0.0.1:{proxy_port}`; the OS
//! redirector forwards captured packets over IPC and does not connect to the proxy directly.

use std::path::{Path, PathBuf};

use sysinfo::{Pid, ProcessesToUpdate, System};

pub fn is_trusted_relay_peer(pid: u32) -> bool {
    evaluate_relay_peer_trust(pid).0
}

/// Human-readable trust outcome for capture diagnostics (`ARGUS_CAPTURE_LOG=1`).
pub fn diagnose_relay_peer(pid: u32) -> String {
    evaluate_relay_peer_trust(pid).1
}

fn evaluate_relay_peer_trust(pid: u32) -> (bool, String) {
    if pid == 0 {
        return (false, "relay peer pid=0".into());
    }
    let mut system = System::new();
    let sys_pid = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);
    let Some(proc) = system.process(sys_pid) else {
        return (false, format!("relay peer pid={pid}: process not found"));
    };
    let Some(exe) = proc.exe() else {
        return (false, format!("relay peer pid={pid}: exe path unavailable"));
    };
    evaluate_relay_exe_trust(&exe.to_string_lossy())
}

pub fn is_trusted_relay_exe_path(exe_path: &str) -> bool {
    evaluate_relay_exe_trust(exe_path).0
}

fn evaluate_relay_exe_trust(exe_path: &str) -> (bool, String) {
    let path = Path::new(exe_path);
    let file_name = relay_exe_basename(exe_path);

    if is_dev_build_path(path) {
        if is_known_relay_exe_name(file_name) {
            return (
                true,
                format!(
                    "relay exe trusted (dev build): {}",
                    path.display()
                ),
            );
        }
        return (
            false,
            format!(
                "relay exe not a known sidecar: {} (basename={file_name:?})",
                path.display()
            ),
        );
    }

    if !is_known_relay_exe_name(file_name) {
        return (
            false,
            format!(
                "relay exe not a known sidecar: {} (basename={file_name:?})",
                path.display()
            ),
        );
    }

    let is_redirector = matches!(
        file_name,
        Some("argus-redirector-linux") | Some("argus-redirector-windows.exe")
    );

    let home = argus_home();
    let base = if is_redirector {
        redirector_dir(&home)
    } else {
        home.clone()
    };
    if is_path_under(&base, path) {
        return (
            true,
            format!(
                "relay exe trusted under {}: {}",
                base.display(),
                path.display()
            ),
        );
    }

    (
        false,
        format!(
            "relay exe outside install tree: exe={} expected_under={} (ARGUS_HOME={})",
            path.display(),
            base.display(),
            home.display()
        ),
    )
}

fn relay_exe_basename(exe_path: &str) -> Option<&str> {
    exe_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
}

fn is_known_relay_exe_name(file_name: Option<&str>) -> bool {
    matches!(
        file_name,
        Some("argus-cli")
            | Some("argus-cli.exe")
            | Some("argus.exe")
            | Some("argus-redirector-linux")
            | Some("argus-redirector-windows.exe")
    )
}

fn is_dev_build_path(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    s.contains("target/debug") || s.contains("target/release")
}

fn is_path_under(base: &Path, candidate: &Path) -> bool {
    #[cfg(windows)]
    {
        let base_s = base.to_string_lossy().replace('/', "\\").to_lowercase();
        let cand_s = candidate.to_string_lossy().replace('/', "\\").to_lowercase();
        let base_trim = base_s.trim_end_matches('\\');
        cand_s == base_trim
            || cand_s.starts_with(&format!("{base_trim}\\"))
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(base)
    }
}

fn argus_home() -> PathBuf {
    if let Ok(home) = std::env::var("ARGUS_HOME") {
        return PathBuf::from(home);
    }
    #[cfg(windows)]
    {
        if let Ok(path) = read_install_path_registry() {
            return path;
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(home) = install_dir_from_exe(&exe) {
                return home;
            }
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("argus");
        }
        return PathBuf::from(r"C:\Program Files\argus");
    }
    #[cfg(unix)]
    {
        for candidate in ["/usr/lib/argus", "/opt/Argus"] {
            let path = PathBuf::from(candidate);
            if path.join("lib").join("argus").exists() {
                return path;
            }
        }
        PathBuf::from("/usr/lib/argus")
    }
}

#[cfg(windows)]
fn install_dir_from_exe(exe: &Path) -> Option<PathBuf> {
    let parent = exe.parent()?;
    if parent.file_name().and_then(|n| n.to_str()) == Some("bin") {
        return parent.parent().map(PathBuf::from);
    }
    Some(parent.to_path_buf())
}

#[cfg(windows)]
fn read_install_path_registry() -> Result<PathBuf, ()> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    read_install_path_from_hive(HKEY_LOCAL_MACHINE)
        .or_else(|_| read_install_path_from_hive(HKEY_CURRENT_USER))
}

#[cfg(windows)]
fn read_install_path_from_hive(
    hive: windows_sys::Win32::System::Registry::HKEY,
) -> Result<PathBuf, ()> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_SZ,
    };

    let subkey: Vec<u16> = "Software\\Argus\0".encode_utf16().collect();
    let value_name: Vec<u16> = "InstallPath\0".encode_utf16().collect();
    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(hive, subkey.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return Err(());
        }
        let mut kind = 0u32;
        let mut size = 0u32;
        if RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        ) != 0
            || kind != REG_SZ
        {
            RegCloseKey(key);
            return Err(());
        }
        let mut buf = vec![0u16; (size as usize).max(2) / 2];
        if RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        ) != 0
        {
            RegCloseKey(key);
            return Err(());
        }
        RegCloseKey(key);
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
    }
}

fn redirector_dir(home: &Path) -> PathBuf {
    home.join("lib").join("argus")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ARGUS_HOME_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_argus_home<F: FnOnce()>(home: &Path, f: F) {
        let _guard = ARGUS_HOME_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("ARGUS_HOME", home.as_os_str());
        f();
        std::env::remove_var("ARGUS_HOME");
    }

    #[test]
    fn accepts_install_paths() {
        let home = std::env::temp_dir().join("argus-trust-test");
        let cli = home.join("lib").join("argus").join("argus-cli.exe");
        let redirector = home
            .join("lib")
            .join("argus")
            .join("argus-redirector-windows.exe");
        with_argus_home(&home, || {
            assert!(is_trusted_relay_exe_path(cli.to_string_lossy().as_ref()));
            assert!(is_trusted_relay_exe_path(
                redirector.to_string_lossy().as_ref()
            ));
        });
    }

    #[test]
    fn accepts_program_files_argus_casing() {
        let home = PathBuf::from(r"C:\Program Files\argus");
        let cli = home.join("bin").join("argus.exe");
        with_argus_home(&home, || {
            let (ok, detail) = evaluate_relay_exe_trust(&cli.to_string_lossy());
            assert!(ok, "{detail}");
        });
    }

    #[test]
    fn accepts_install_path_case_insensitive() {
        let home = PathBuf::from(r"C:\Program Files\Argus");
        let cli = PathBuf::from(r"c:\program files\argus\bin\argus.exe");
        with_argus_home(&home, || {
            let (ok, detail) = evaluate_relay_exe_trust(&cli.to_string_lossy());
            assert!(ok, "{detail}");
        });
    }

    #[test]
    fn accepts_dev_build_paths() {
        assert!(is_trusted_relay_exe_path(
            r"E:\Projects\argus-project\argus\target\debug\argus-cli.exe"
        ));
    }

    #[test]
    fn rejects_wrong_directory_same_basename() {
        assert!(!is_trusted_relay_exe_path(
            r"C:\Windows\System32\argus-cli.exe"
        ));
        assert!(!is_trusted_relay_exe_path(
            r"C:\Users\attacker\argus-cli.exe"
        ));
        assert!(!is_trusted_relay_exe_path(r"C:\Windows\System32\node.exe"));
    }
}
