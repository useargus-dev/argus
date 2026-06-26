//! Temporary stderr trace for `argus run` transparent capture (debug builds or `ARGUS_CAPTURE_LOG=1`).
//!
//! Relay/auth lines are also appended to `~/.argus/capture-trace.log` so the desktop app
//! (no console) still leaves a trail during installed-build debugging.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static TRACE_FILE: Mutex<()> = Mutex::new(());

/// True when capture trace lines should be printed.
pub fn enabled() -> bool {
    match std::env::var("ARGUS_CAPTURE_LOG") {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => cfg!(debug_assertions),
    }
}

fn trace_log_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let home = std::env::var("USERPROFILE").ok()?;
    #[cfg(not(windows))]
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".argus").join("capture-trace.log"))
}

fn should_persist_to_file(component: &str) -> bool {
    matches!(
        component,
        "relay-auth" | "peer-trust" | "proxy" | "gate" | "run" | "relay" | "redirector"
    )
}

fn append_trace_file(component: &str, message: &str) {
    let Some(path) = trace_log_path() else {
        return;
    };
    let Ok(_guard) = TRACE_FILE.lock() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = writeln!(
        file,
        "[argus-capture][{component}] {message}",
    );
}

/// Print a capture trace line to stderr (`[argus-capture] …`) and optionally `~/.argus/capture-trace.log`.
pub fn log(component: &str, message: impl AsRef<str>) {
    let message = message.as_ref();
    if enabled() {
        eprintln!("[argus-capture][{component}] {message}");
    }
    if enabled() || should_persist_to_file(component) {
        append_trace_file(component, message);
    }
}

/// Path to the on-disk trace file (for user-facing hints).
pub fn trace_log_hint() -> Option<String> {
    trace_log_path().map(|p| p.display().to_string())
}
