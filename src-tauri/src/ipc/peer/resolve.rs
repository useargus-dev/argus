use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::error::{AppError, AppResult};
use crate::user_messages;

use super::machine_id;
use super::proc_info;

/// Complete peer fingerprint derived from OS process inspection.
#[derive(Debug, Clone)]
pub struct VerifiedClient {
    pub pid: u32,
    pub exe_path: String,
    pub cwd: String,
    pub cwd_verified: bool,
    pub run_args: String,
    pub process_name: String,
    pub uid: String,
    pub machine_id: String,
    pub git_remote: Option<String>,
    pub fingerprint: String,
}

impl VerifiedClient {
    pub fn from_pid(pid: u32, fallback_cwd: Option<&str>) -> AppResult<Self> {
        if pid == 0 {
            return Err(AppError::message(
                "PEER_RESOLVE",
                user_messages::peer_resolve("invalid peer pid 0"),
            ));
        }

        let mut system = System::new();
        let sys_pid = Pid::from_u32(pid);
        system.refresh_processes(ProcessesToUpdate::Some(&[sys_pid]), true);

        let proc = system.process(sys_pid).ok_or_else(|| {
            AppError::message(
                "PEER_RESOLVE",
                user_messages::peer_resolve(format!(
                    "process {pid} not found (exited before inspection?)"
                )),
            )
        })?;

        let exe_path = proc
            .exe()
            .map(|p| p.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AppError::message(
                    "PEER_RESOLVE",
                    user_messages::peer_resolve(format!(
                        "could not read executable path for pid {pid}"
                    )),
                )
            })?;

        // Use native OS APIs for cmd line and cwd (sysinfo is unreliable on Windows)
        let native_info = proc_info::read_proc_info(pid);

        let (cwd, cwd_verified) = {
            // Try native API first, then sysinfo, then fallback
            let native_cwd = native_info.as_ref().and_then(|i| i.cwd.clone());
            let sysinfo_cwd = proc.cwd().map(|p| p.to_string_lossy().into_owned()).filter(|s| !s.is_empty());

            if let Some(d) = native_cwd {
                (d, true)
            } else if let Some(d) = sysinfo_cwd {
                (d, true)
            } else if let Some(fb) = fallback_cwd.filter(|s| !s.is_empty()) {
                (fb.to_string(), false)
            } else {
                let parent = PathBuf::from(&exe_path)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if parent.is_empty() {
                    return Err(AppError::message(
                        "PEER_RESOLVE",
                        user_messages::peer_resolve(format!(
                            "could not determine working directory for pid {pid}"
                        )),
                    ));
                }
                (parent, false)
            }
        };

        let run_args = {
            // Try native API first, then sysinfo
            let native_cmd = native_info.as_ref().map(|i| i.cmd_line.clone()).filter(|s| !s.is_empty());
            let sysinfo_cmd: String = proc.cmd().iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" ");

            native_cmd.unwrap_or(sysinfo_cmd)
        };

        let process_name = PathBuf::from(&exe_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("pid-{pid}"));

        let uid = current_uid();
        let mid = machine_id::read_machine_id();
        let git_remote = read_git_remote(&cwd);

        let fingerprint = compute_fingerprint(&mid, git_remote.as_deref(), &cwd, &exe_path, &uid, &run_args);


        Ok(Self {
            pid,
            exe_path,
            cwd,
            cwd_verified,
            run_args,
            process_name,
            uid,
            machine_id: mid,
            git_remote,
            fingerprint,
        })
    }
}

fn compute_fingerprint(
    machine_id: &str,
    git_remote: Option<&str>,
    cwd: &str,
    exe_path: &str,
    uid: &str,
    run_args: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(b"|");
    hasher.update(git_remote.unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(cwd.replace('\\', "/").to_lowercase().as_bytes());
    hasher.update(b"|");
    hasher.update(exe_path.replace('\\', "/").to_lowercase().as_bytes());
    hasher.update(b"|");
    hasher.update(uid.as_bytes());
    hasher.update(b"|");
    hasher.update(run_args.replace('\\', "/").to_lowercase().as_bytes());
    hex::encode(hasher.finalize())
}

fn read_git_remote(cwd: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", cwd, "remote", "get-url", "origin"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if remote.is_empty() {
        None
    } else {
        Some(remote)
    }
}

#[cfg(unix)]
fn current_uid() -> String {
    unsafe { libc::getuid() }.to_string()
}

#[cfg(windows)]
fn current_uid() -> String {
    whoami::username()
}

#[cfg(not(any(unix, windows)))]
fn current_uid() -> String {
    "unknown".to_string()
}
