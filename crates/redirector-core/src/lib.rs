//! Resolve bundled redirector binaries and start/stop capture.

use std::path::{Path, PathBuf};

use anyhow::Result;
#[cfg(target_os = "linux")]
use intercept::{start_linux_redirector, RedirectorHandle};
#[cfg(windows)]
use intercept::{start_windows_redirector, RedirectorHandle};

#[cfg(windows)]
mod registry_win;

pub fn argus_home() -> PathBuf {
    if let Ok(home) = std::env::var("ARGUS_HOME") {
        return PathBuf::from(home);
    }
    #[cfg(windows)]
    {
        if let Ok(path) = registry_win::read_install_path_registry() {
            return path;
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("Argus");
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // Dev layout: target/release/argus-cli + lib/argus/
            let lib = parent.join("lib").join("argus");
            if lib.exists() {
                return parent.to_path_buf();
            }
            // Tauri bundle: Resources/lib/argus
            let resources = parent.join("lib").join("argus");
            if resources.exists() {
                return parent.to_path_buf();
            }
        }
    }
    #[cfg(unix)]
    {
        return PathBuf::from("/opt/Argus");
    }
    #[cfg(windows)]
    {
        PathBuf::from(r"C:\Program Files\Argus")
    }
}

pub fn redirector_dir() -> PathBuf {
    argus_home().join("lib").join("argus")
}

pub fn linux_redirector_path() -> PathBuf {
    redirector_dir().join("argus-redirector-linux")
}

pub fn windows_redirector_path() -> PathBuf {
    redirector_dir().join("argus-redirector-windows.exe")
}

pub fn privilege_hint() -> Option<String> {
    intercept::elevation_notice()
}

pub async fn start_redirector(
    target_port: u16,
    redirector_override: Option<&Path>,
) -> Result<RedirectorHandle> {
    #[cfg(target_os = "linux")]
    {
        let path = redirector_override
            .map(Path::to_path_buf)
            .unwrap_or_else(linux_redirector_path);
        let handle = start_linux_redirector(path, target_port).await?;
        Ok(handle)
    }

    #[cfg(windows)]
    {
        let path = redirector_override
            .map(Path::to_path_buf)
            .unwrap_or_else(windows_redirector_path);
        let handle = start_windows_redirector(path, target_port).await?;
        Ok(handle)
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (target_port, redirector_override);
        anyhow::bail!("argus run is not supported on this platform yet")
    }
}

pub fn intercept_spec_for_pid(root_pid: u32) -> String {
    root_pid.to_string()
}
