//! Cross-platform command resolution for `argus run` child processes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::{Child, Command};

/// Build a child process command: resolve the program, inherit cwd, merge sandbox env.
pub fn build_run_command(
    argv: &[String],
    sandbox_env: &HashMap<String, String>,
) -> Result<Command> {
    Ok(base_command(argv, sandbox_env)?)
}

pub struct PausedSpawn {
    pub child: Child,
    gate: Option<SpawnGate>,
}

enum SpawnGate {
    #[cfg(unix)]
    UnixWrite(std::os::fd::OwnedFd),
    #[cfg(windows)]
    WindowsSuspended,
}

/// Spawn the child paused until intercept + PID registration complete.
pub fn spawn_paused(
    argv: &[String],
    sandbox_env: &HashMap<String, String>,
) -> Result<PausedSpawn> {
    #[cfg(unix)]
    {
        return spawn_paused_unix(argv, sandbox_env);
    }
    #[cfg(windows)]
    {
        return spawn_paused_windows(argv, sandbox_env);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let mut cmd = base_command(argv, sandbox_env)?;
        let child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn `{}`", argv.first().unwrap_or(&"?".into())))?;
        Ok(PausedSpawn { child, gate: None })
    }
}

pub fn release_paused_child(paused: &mut PausedSpawn) -> Result<()> {
    match paused.gate.take() {
        #[cfg(unix)]
        Some(SpawnGate::UnixWrite(fd)) => {
            use std::io::Write;
            use std::os::fd::{FromRawFd, IntoRawFd};
            let raw = fd.into_raw_fd();
            let mut file = unsafe { std::fs::File::from_raw_fd(raw) };
            file.write_all(&[1u8])
                .context("failed to release spawn gate")?;
            Ok(())
        }
        #[cfg(windows)]
        Some(SpawnGate::WindowsSuspended) => {
            let pid = paused
                .child
                .id()
                .context("paused child has no pid")?;
            resume_suspended_process(pid)
        }
        None => Ok(()),
    }
}

fn base_command(argv: &[String], sandbox_env: &HashMap<String, String>) -> Result<Command> {
    let program = argv.first().context("empty command")?;
    let resolved = resolve_program(program).with_context(|| {
        format!(
            "could not resolve program `{program}` (use an absolute path or ensure it is on PATH)"
        )
    })?;

    let cwd = std::env::current_dir().context("could not read current working directory")?;

    let mut cmd = Command::new(&resolved);
    cmd.args(&argv[1..]);
    cmd.current_dir(&cwd);
    for (key, value) in sandbox_env {
        if key.contains('\0') || value.contains('\0') {
            continue;
        }
        cmd.env(key, value);
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    Ok(cmd)
}

#[cfg(unix)]
fn spawn_paused_unix(
    argv: &[String],
    sandbox_env: &HashMap<String, String>,
) -> Result<PausedSpawn> {
    use std::os::fd::{AsRawFd, IntoRawFd};
    use std::os::unix::process::CommandExt;

    use nix::fcntl::{fcntl, FcntlArg, FdFlag};

    let mut cmd = base_command(argv, sandbox_env)?;
    let (read_fd, write_fd) = nix::unistd::pipe().context("pipe for spawn gate")?;

    // pipe() may set O_CLOEXEC; clear it on the read end so the child inherits it.
    fcntl(read_fd.as_raw_fd(), FcntlArg::F_SETFD(FdFlag::empty()))
        .context("clear FD_CLOEXEC on spawn gate")?;

    let read_raw = read_fd.into_raw_fd();
    unsafe {
        cmd.pre_exec(move || {
            let mut buf = [0u8; 1];
            loop {
                match nix::unistd::read(read_raw, &mut buf) {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "spawn gate closed before release",
                        ));
                    }
                    Ok(_) => break,
                    Err(nix::errno::Errno::EINTR) => continue,
                    Err(e) => return Err(std::io::Error::from(e)),
                }
            }
            let _ = nix::unistd::close(read_raw);
            Ok(())
        });
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn `{}`", argv.first().unwrap_or(&"?".into())))?;

    // Close parent's copy of the read end (child has its own).
    unsafe {
        let _ = nix::unistd::close(read_raw);
    }
    // write_fd keeps CLOEXEC so the child does not inherit a second writer.
    let _ = write_fd.as_raw_fd();

    Ok(PausedSpawn {
        child,
        gate: Some(SpawnGate::UnixWrite(write_fd)),
    })
}

#[cfg(windows)]
fn spawn_paused_windows(
    argv: &[String],
    sandbox_env: &HashMap<String, String>,
) -> Result<PausedSpawn> {
    let mut cmd = base_command(argv, sandbox_env)?;
    const CREATE_SUSPENDED: u32 = 0x0000_0004;
    cmd.creation_flags(CREATE_SUSPENDED);
    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn `{}`", argv.first().unwrap_or(&"?".into())))?;
    Ok(PausedSpawn {
        child,
        gate: Some(SpawnGate::WindowsSuspended),
    })
}

#[cfg(windows)]
fn resume_suspended_process(pid: u32) -> Result<()> {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{
        OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
    };

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snap == INVALID_HANDLE_VALUE {
            anyhow::bail!("CreateToolhelp32Snapshot failed while resuming pid {pid}");
        }
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut resumed = 0u32;
        if Thread32First(snap, &mut entry) != 0 {
            loop {
                if entry.th32OwnerProcessID == pid {
                    let th = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !th.is_null() {
                        let _ = ResumeThread(th);
                        CloseHandle(th);
                        resumed += 1;
                    }
                }
                if Thread32Next(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        if resumed == 0 {
            anyhow::bail!("no threads resumed for pid {pid}");
        }
    }
    Ok(())
}

/// Resolve an executable: absolute/relative paths first, then PATH (+ PATHEXT on Windows).
pub fn resolve_program(program: &str) -> Result<PathBuf> {
    let path = Path::new(program);

    if path.is_absolute() || looks_like_path(program) {
        return resolve_existing_file(path);
    }

    if path.is_file() {
        return Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    }

    search_path(program)
}

fn looks_like_path(program: &str) -> bool {
    program.contains('/') || program.contains('\\') || program.starts_with('.')
}

fn resolve_existing_file(path: &Path) -> Result<PathBuf> {
    if path.is_file() {
        return Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    }

    #[cfg(windows)]
    if path.extension().is_none() {
        for ext in windows_pathext() {
            let ext_clean = ext.trim_start_matches('.');
            let candidate = path.with_extension(ext_clean);
            if candidate.is_file() {
                return Ok(candidate
                    .canonicalize()
                    .unwrap_or_else(|_| candidate.to_path_buf()));
            }
        }
    }

    anyhow::bail!("executable not found: {}", path.display())
}

fn search_path(program: &str) -> Result<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    let extensions = windows_search_extensions(program);
    #[cfg(not(windows))]
    let extensions = vec![String::new()];

    #[cfg(windows)]
    let sep = ';';
    #[cfg(not(windows))]
    let sep = ':';

    for dir in path_var.split(sep).filter(|d| !d.is_empty()) {
        let dir = Path::new(dir.trim_matches('"'));
        for ext in &extensions {
            let candidate = if ext.is_empty() {
                dir.join(program)
            } else {
                dir.join(format!("{program}{ext}"))
            };
            if candidate.is_file() {
                return Ok(candidate
                    .canonicalize()
                    .unwrap_or_else(|_| candidate.to_path_buf()));
            }
        }
    }

    anyhow::bail!("program not found: {program}")
}

#[cfg(windows)]
fn windows_search_extensions(program: &str) -> Vec<String> {
    if Path::new(program).extension().is_some() {
        return vec![String::new()];
    }
    windows_pathext()
}

#[cfg(windows)]
fn windows_pathext() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC;".into())
        .split(';')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_absolute_existing_file() {
        #[cfg(windows)]
        let p = PathBuf::from(r"C:\Windows\System32\cmd.exe");
        #[cfg(not(windows))]
        let p = PathBuf::from("/bin/sh");
        if p.is_file() {
            let resolved = resolve_program(p.to_str().unwrap()).unwrap();
            assert!(resolved.is_file());
        }
    }

    #[test]
    fn bare_name_searches_path() {
        #[cfg(windows)]
        let name = "cmd.exe";
        #[cfg(not(windows))]
        let name = "sh";
        if resolve_program(name).is_ok() {
            assert!(resolve_program(name).unwrap().is_file());
        }
    }
}
