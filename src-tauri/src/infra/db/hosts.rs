//! Host allowlist normalization and matching for bucket proxy.

use crate::error::{AppError, AppResult};

/// Sentinel value: allow any request host for proxy rewrite on this mapping.
pub const ALLOW_ALL_SENTINEL: &str = "*";

/// Strip scheme, port, path; lowercase host.
pub fn normalize_host(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    for prefix in ["https://", "http://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    if let Some((host, _)) = s.split_once('/') {
        s = host.to_string();
    }
    if let Some((host, _)) = s.split_once(':') {
        s = host.to_string();
    }
    s.trim().to_string()
}

pub fn normalize_host_list(hosts: &[String]) -> Vec<String> {
    let mut out: Vec<String> = hosts
        .iter()
        .map(|h| normalize_host(h))
        .filter(|h| !h.is_empty())
        .collect();
    if out.iter().any(|h| h == ALLOW_ALL_SENTINEL) {
        return vec![ALLOW_ALL_SENTINEL.to_string()];
    }
    out.sort();
    out.dedup();
    out
}

pub fn parse_allowed_hosts_json(json: &str) -> AppResult<Vec<String>> {
    let hosts: Vec<String> = serde_json::from_str(json)
        .map_err(|e| AppError::message("DB_ERROR", format!("invalid allowed_hosts: {e}")))?;
    Ok(normalize_host_list(&hosts))
}

pub fn allowed_hosts_to_json(hosts: &[String]) -> AppResult<String> {
    serde_json::to_string(&normalize_host_list(hosts))
        .map_err(|e| AppError::message("DB_ERROR", e.to_string()))
}

/// True if `request_host` matches any allowlist entry (exact or subdomain suffix).
/// A list containing [`ALLOW_ALL_SENTINEL`] permits any non-empty host.
pub fn host_is_allowed(request_host: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }
    if allowed.iter().any(|entry| entry == ALLOW_ALL_SENTINEL) {
        return !normalize_host(request_host).is_empty();
    }
    let h = normalize_host(request_host);
    if h.is_empty() {
        return false;
    }
    allowed.iter().any(|entry| {
        h == *entry || h.ends_with(&format!(".{entry}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_scheme() {
        assert_eq!(normalize_host("https://API.OpenAI.com/v1"), "api.openai.com");
    }

    #[test]
    fn subdomain_match() {
        let allowed = vec!["openai.com".to_string()];
        assert!(host_is_allowed("api.openai.com", &allowed));
        assert!(!host_is_allowed("evil.com", &allowed));
    }

    #[test]
    fn allow_all_sentinel() {
        let allowed = vec![ALLOW_ALL_SENTINEL.to_string()];
        assert!(host_is_allowed("api.openai.com", &allowed));
        assert!(host_is_allowed("anything.example", &allowed));
        assert!(!host_is_allowed("", &allowed));
    }

    #[test]
    fn normalize_host_list_allow_all_wins() {
        assert_eq!(
            normalize_host_list(&["openai.com".to_string(), "*".to_string()]),
            vec!["*".to_string()]
        );
    }
}
