//! Temporary stderr trace for `argus run` transparent capture (debug builds or `ARGUS_CAPTURE_LOG=1`).

/// True when capture trace lines should be printed.
pub fn enabled() -> bool {
    match std::env::var("ARGUS_CAPTURE_LOG") {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => cfg!(debug_assertions),
    }
}

/// Print a capture trace line to stderr (`[argus-capture] …`).
pub fn log(component: &str, message: impl AsRef<str>) {
    if enabled() {
        eprintln!("[argus-capture][{component}] {}", message.as_ref());
    }
}
