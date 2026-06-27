//! User-facing IPC error and status messages for SDK clients.

pub fn invalid_request_json(detail: &str) -> String {
    format!(
        "Invalid IPC request JSON: {detail}. Send one JSON object per line with request_id, bucket_id, and client_token."
    )
}

pub fn locked_signed_out() -> &'static str {
    "Argus is not signed in. Sign in to the Argus app and retry."
}

pub fn bucket_not_found(bucket_id: &str) -> String {
    format!(
        "Bucket '{bucket_id}' was not found. Verify ARGUS_BUCKET_ID in your .env matches a bucket in Argus."
    )
}

pub fn bucket_inactive(bucket_name: &str) -> String {
    format!(
        "Bucket '{bucket_name}' is paused. Activate it in Argus (Buckets page or system tray) and retry."
    )
}

pub fn invalid_token(bucket_name: &str) -> String {
    format!(
        "Client token rejected for bucket '{bucket_name}'. Regenerate the token in Argus bucket settings and update ARGUS_BUCKET_TOKEN in your .env."
    )
}

pub fn invalid_token_generic() -> &'static str {
    "Client token rejected. Regenerate the token in Argus bucket settings and update ARGUS_BUCKET_TOKEN."
}

pub fn approval_timeout() -> &'static str {
    "Access approval timed out after 120 seconds. Open Argus, approve the pending client request in the Requests window, and retry."
}

pub fn approval_denied() -> &'static str {
    "Access denied. Approve this client in Argus (Requests window) and retry."
}

pub fn proxy_port_missing(bucket_name: &str) -> String {
    format!(
        "Bucket '{bucket_name}' has proxy enabled but no proxy port is allocated. Toggle proxy off and on in Argus bucket settings, then retry."
    )
}

pub fn proxy_disabled(bucket_name: &str) -> String {
    format!(
        "Enable Argus Proxy on bucket '{bucket_name}' in the Argus app before using argus run."
    )
}

pub fn peer_resolve(detail: impl Into<String>) -> String {
    format!(
        "Could not identify the connecting process: {}. Ensure Argus is running and retry from a normal terminal or IDE (not a stale subprocess).",
        detail.into()
    )
}
