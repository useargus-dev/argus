use std::time::{Duration, Instant};

/// Stage log for auth flows. Enabled in debug builds or when `ARGUS_AUTH_TRACE=1`.
pub fn auth_stage(flow: &str, stage: &str, elapsed: Option<Duration>) {
    if !auth_trace_enabled() {
        return;
    }
    match elapsed {
        Some(d) => eprintln!(
            "[argus:{flow}] {stage} (+{} ms)",
            d.as_millis()
        ),
        None => eprintln!("[argus:{flow}] {stage}"),
    }
}

pub fn auth_trace_enabled() -> bool {
    cfg!(debug_assertions) || std::env::var("ARGUS_AUTH_TRACE").as_deref() == Ok("1")
}

pub struct AuthTimer {
    flow: &'static str,
    start: Instant,
}

impl AuthTimer {
    pub fn start(flow: &'static str) -> Self {
        auth_stage(flow, "start", None);
        Self {
            flow,
            start: Instant::now(),
        }
    }

    pub fn stage(&self, stage: &str) {
        auth_stage(self.flow, stage, Some(self.start.elapsed()));
    }

    pub fn done(&self) {
        auth_stage(self.flow, "done", Some(self.start.elapsed()));
    }
}
