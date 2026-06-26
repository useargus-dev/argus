#[cfg(windows)]
use crate::registry_win;

use std::path::{Path, PathBuf};

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
            for home in install_dir_candidates(parent) {
                if home.join("lib").join("argus").exists() {
                    return home;
                }
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

/// Candidate install roots when resolving from the running executable path.
fn install_dir_candidates(exe_parent: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![exe_parent.to_path_buf()];
    if exe_parent.file_name().and_then(|n| n.to_str()) == Some("bin") {
        if let Some(grandparent) = exe_parent.parent() {
            candidates.push(grandparent.to_path_buf());
        }
    }
    candidates
}

pub fn redirector_dir() -> PathBuf {
    argus_home().join("lib").join("argus")
}

/// Installed sidecar path under `dir` (`.exe` on Windows).
pub fn resolve_sidecar_binary(dir: &Path, base_name: &str) -> PathBuf {
    #[cfg(windows)]
    {
        dir.join(format!("{base_name}.exe"))
    }
    #[cfg(not(windows))]
    {
        dir.join(base_name)
    }
}

pub fn linux_redirector_path() -> PathBuf {
    resolve_sidecar_binary(&redirector_dir(), "argus-redirector-linux")
}

pub fn windows_redirector_path() -> PathBuf {
    resolve_sidecar_binary(&redirector_dir(), "argus-redirector-windows")
}

pub fn intercept_spec_for_pid(root_pid: u32) -> String {
    root_pid.to_string()
}

pub fn intercept_spec_for_pids(pids: &[u32]) -> String {
    pids.iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub async fn start_redirector(
    target_port: u16,
    redirector_override: Option<&Path>,
    relay_secret: Option<[u8; 32]>,
) -> anyhow::Result<crate::RedirectorHandle> {
    #[cfg(target_os = "linux")]
    {
        let path = redirector_override
            .map(Path::to_path_buf)
            .unwrap_or_else(linux_redirector_path);
        crate::start_linux_redirector(path, target_port, relay_secret).await
    }

    #[cfg(windows)]
    {
        let path = redirector_override
            .map(Path::to_path_buf)
            .unwrap_or_else(windows_redirector_path);
        crate::start_windows_redirector(path, target_port, relay_secret).await
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (target_port, redirector_override, relay_secret);
        anyhow::bail!("argus run is not supported on this platform yet")
    }
}
