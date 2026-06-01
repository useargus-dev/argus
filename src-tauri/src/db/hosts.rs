//! Host allowlist normalization and matching for bucket proxy.

use crate::error::{AppError, AppResult};

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
pub fn host_is_allowed(request_host: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
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
}
