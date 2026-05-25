use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    pub request_id: String,
    pub bucket_id: String,
    pub client_token: String,
    /// Optional fallback cwd (used on Windows when OS can't read peer cwd).
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum IpcResponse {
    Ok {
        request_id: String,
        env: HashMap<String, String>,
    },
    Denied {
        request_id: String,
        message: String,
    },
    Locked {
        request_id: String,
        message: String,
    },
    Error {
        request_id: String,
        code: String,
        message: String,
    },
}

impl IpcResponse {
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"status":"error","request_id":"","code":"SERIALIZE_ERROR","message":"response encode failed"}"#
                .to_string()
        })
    }
}
