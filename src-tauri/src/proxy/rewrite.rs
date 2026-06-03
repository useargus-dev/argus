use bytes::Bytes;

use crate::infra::db::bucket_mappings::ProxyRewriteEntry;

/// Replace header values that exactly match a placeholder (or Bearer {placeholder}).
pub fn rewrite_headers(
    headers: &mut http::HeaderMap,
    entries: &[ProxyRewriteEntry],
) -> Option<String> {
    let mut used_label: Option<String> = None;
    for entry in entries {
        for (_name, value) in headers.iter_mut() {
            if let Ok(s) = value.to_str() {
                let new_val = if s == entry.placeholder {
                    Some(entry.secret_plain.clone())
                } else if let Some(rest) = s.strip_prefix("Bearer ") {
                    if rest == entry.placeholder {
                        Some(format!("Bearer {}", entry.secret_plain))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(v) = new_val {
                    *value = http::HeaderValue::from_str(&v)
                        .unwrap_or_else(|_| value.clone());
                    used_label = Some(entry.env_label.clone());
                }
            }
        }
    }
    used_label
}

/// Replace placeholder substrings in UTF-8 request bodies (e.g. JSON fields).
pub fn rewrite_body(body: &Bytes, entries: &[ProxyRewriteEntry]) -> Bytes {
    let Ok(text) = std::str::from_utf8(body) else {
        return body.clone();
    };
    let mut out = text.to_string();
    let mut changed = false;
    for entry in entries {
        if out.contains(&entry.placeholder) {
            out = out.replace(&entry.placeholder, &entry.secret_plain);
            changed = true;
        }
    }
    if changed {
        Bytes::from(out)
    } else {
        body.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::AUTHORIZATION;

    #[test]
    fn rewrites_bearer_placeholder() {
        let entries = vec![ProxyRewriteEntry {
            env_label: "OPENAI_API_KEY".to_string(),
            placeholder: "argus-proxy-abc".to_string(),
            secret_plain: "sk-real".to_string(),
        }];
        let mut headers = http::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            http::HeaderValue::from_static("Bearer argus-proxy-abc"),
        );
        let label = rewrite_headers(&mut headers, &entries);
        assert_eq!(label.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(
            headers.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer sk-real"
        );
    }

    #[test]
    fn rewrites_json_body_placeholder() {
        let entries = vec![ProxyRewriteEntry {
            env_label: "API_KEY".to_string(),
            placeholder: "argus-proxy-xyz".to_string(),
            secret_plain: "secret-value".to_string(),
        }];
        let body = Bytes::from(r#"{"api_key":"argus-proxy-xyz"}"#);
        let out = rewrite_body(&body, &entries);
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            r#"{"api_key":"secret-value"}"#
        );
    }
}
