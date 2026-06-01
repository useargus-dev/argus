use http::HeaderMap;

use crate::db::bucket_mappings::ProxyRewriteEntry;
use crate::proxy::debug_log::HeaderRewrite;

/// Replace header values that exactly match a placeholder (or Bearer {placeholder}).
pub fn rewrite_headers(
    headers: &mut HeaderMap,
    entries: &[ProxyRewriteEntry],
) -> (Option<String>, Vec<HeaderRewrite>) {
    let mut used_label: Option<String> = None;
    let mut rewrites = Vec::new();
    for entry in entries {
        for (name, value) in headers.iter_mut() {
            if let Ok(s) = value.to_str() {
                let (new_val, before) = if s == entry.placeholder {
                    (
                        Some(entry.secret_plain.clone()),
                        s.to_string(),
                    )
                } else if let Some(rest) = s.strip_prefix("Bearer ") {
                    if rest == entry.placeholder {
                        (
                            Some(format!("Bearer {}", entry.secret_plain)),
                            s.to_string(),
                        )
                    } else {
                        (None, String::new())
                    }
                } else {
                    (None, String::new())
                };
                if let Some(v) = new_val {
                    rewrites.push(HeaderRewrite {
                        header: name.to_string(),
                        env_label: entry.env_label.clone(),
                        before,
                        after: v.clone(),
                    });
                    *value = http::HeaderValue::from_str(&v)
                        .unwrap_or_else(|_| value.clone());
                    used_label = Some(entry.env_label.clone());
                }
            }
        }
    }
    (used_label, rewrites)
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
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            http::HeaderValue::from_static("Bearer argus-proxy-abc"),
        );
        let (label, rewrites) = rewrite_headers(&mut headers, &entries);
        assert_eq!(label.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(rewrites.len(), 1);
        assert_eq!(
            headers.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer sk-real"
        );
    }
}
