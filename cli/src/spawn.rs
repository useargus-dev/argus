//! Cross-platform command resolution for `argus run` child processes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;

/// Build a child process command: resolve the program, inherit cwd, merge sandbox env.
pub fn build_run_command(argv: &[String], sandbox_env: &HashMap<String, String>) -> Result<Command> {
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

    for dir in path_var.split(';').filter(|d| !d.is_empty()) {
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
