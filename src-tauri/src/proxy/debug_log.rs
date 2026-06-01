use std::sync::atomic::{AtomicBool, Ordering};

use http::HeaderMap;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Call once per process; also refreshed when env var is set at request time.
pub fn proxy_debug_enabled() -> bool {
    if ENABLED.load(Ordering::Relaxed) {
        return true;
    }
    let on = std::env::var("ARGUS_PROXY_DEBUG")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if on {
        ENABLED.store(true, Ordering::Relaxed);
    }
    on
}

pub fn redact_value(value: &str, reveal: bool) -> String {
    if reveal {
        return value.to_string();
    }
    if value.len() <= 8 {
        return "***".to_string();
    }
    format!("{}…{}", &value[..4], &value[value.len().saturating_sub(4)..])
}

pub fn format_headers(headers: &HeaderMap, reveal_secrets: bool) -> String {
    let mut lines = Vec::new();
    for (name, value) in headers.iter() {
        let v = value.to_str().unwrap_or("<binary>");
        let is_sensitive = name.as_str().eq_ignore_ascii_case("authorization")
            || name.as_str().eq_ignore_ascii_case("proxy-authorization")
            || name.as_str().contains("api-key")
            || name.as_str().contains("token");
        let shown = if is_sensitive {
            redact_value(v, reveal_secrets)
        } else {
            v.to_string()
        };
        lines.push(format!("  {}: {}", name.as_str(), shown));
    }
    lines.join("\n")
}

pub struct HeaderRewrite {
    pub header: String,
    pub env_label: String,
    pub before: String,
    pub after: String,
}

pub fn log_connect(bucket_id: &str, target: &str, pid: Option<u32>, headers: &[(String, String)]) {
    if !proxy_debug_enabled() {
        return;
    }
    eprintln!("[argus-proxy] CONNECT bucket={bucket_id} target={target} pid={pid:?}");
    for (k, v) in headers {
        let shown = if k.eq_ignore_ascii_case("proxy-authorization") {
            redact_value(v, false)
        } else {
            v.clone()
        };
        eprintln!("[argus-proxy]   {k}: {shown}");
    }
}

pub fn log_gate_result(bucket_id: &str, host: &str, allowed: bool, grant_ok: bool, rewrite_count: usize) {
    if !proxy_debug_enabled() {
        return;
    }
    eprintln!(
        "[argus-proxy] gate bucket={bucket_id} host={host} allowed={allowed} grant_ok={grant_ok} rewrite_entries={rewrite_count}"
    );
}

pub fn log_incoming_request(bucket_id: &str, host: &str, method: &str, uri: &str, headers: &HeaderMap) {
    if !proxy_debug_enabled() {
        return;
    }
    let reveal = proxy_debug_enabled();
    eprintln!(
        "[argus-proxy] >>> incoming bucket={bucket_id} {method} https://{host}{uri}"
    );
    eprintln!("{}", format_headers(headers, reveal));
}

pub fn log_rewrites(bucket_id: &str, rewrites: &[HeaderRewrite]) {
    if !proxy_debug_enabled() {
        return;
    }
    if rewrites.is_empty() {
        eprintln!("[argus-proxy] rewrite bucket={bucket_id} (no placeholder headers matched)");
        return;
    }
    let reveal = proxy_debug_enabled();
    for r in rewrites {
        eprintln!(
            "[argus-proxy] rewrite bucket={bucket_id} mapping={} header={}",
            r.env_label, r.header
        );
        eprintln!(
            "[argus-proxy]   before: {}",
            redact_value(&r.before, reveal)
        );
        eprintln!(
            "[argus-proxy]   after:  {}",
            redact_value(&r.after, reveal)
        );
    }
}

pub fn log_upstream_request(
    bucket_id: &str,
    method: &str,
    uri: &str,
    headers: &HeaderMap,
    body_len: usize,
) {
    if !proxy_debug_enabled() {
        return;
    }
    let reveal = proxy_debug_enabled();
    eprintln!(
        "[argus-proxy] >>> upstream bucket={bucket_id} {method} {uri} body_len={body_len}"
    );
    eprintln!("{}", format_headers(headers, reveal));
}

pub fn log_upstream_response(bucket_id: &str, status: u16, header_preview: &str, body_len: usize, ms: u64) {
    if !proxy_debug_enabled() {
        return;
    }
    eprintln!(
        "[argus-proxy] <<< response bucket={bucket_id} status={status} body_len={body_len} elapsed_ms={ms}"
    );
    for line in header_preview.lines().take(20) {
        if !line.is_empty() {
            eprintln!("[argus-proxy]   {line}");
        }
    }
}

pub fn log_denied(bucket_id: &str, reason: &str, host: &str) {
    if !proxy_debug_enabled() {
        return;
    }
    eprintln!("[argus-proxy] denied bucket={bucket_id} host={host} reason={reason}");
}
