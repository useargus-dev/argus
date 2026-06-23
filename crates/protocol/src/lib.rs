//! Shared IPC types for Argus desktop and CLI sidecar.

pub mod relay_frame;
pub mod capture_log;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// v3 env fetch (no `type` field).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpcFetchEnvRequest {
    pub request_id: String,
    pub bucket_id: String,
    pub client_token: String,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxCreateRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_id: String,
    pub bucket_id: String,
    pub client_token: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub command_preview: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxRegisterPidsRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_id: String,
    pub session_id: String,
    pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxRevokeRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub request_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcSandboxRequest {
    SandboxCreate {
        request_id: String,
        bucket_id: String,
        client_token: String,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        command_preview: Option<String>,
    },
    SandboxRegisterPids {
        request_id: String,
        session_id: String,
        pids: Vec<u32>,
    },
    SandboxRevoke {
        request_id: String,
        session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfigPayload {
    pub enabled: bool,
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
    pub ca_bundle_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum IpcResponse {
    Ok {
        request_id: String,
        env: HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proxy: Option<ProxyConfigPayload>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "sessionId")]
        session_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "proxyPort")]
        proxy_port: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "expiresAt")]
        expires_at: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "caBundlePath")]
        ca_bundle_path: Option<String>,
    },
    Denied {
        request_id: String,
        code: String,
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

#[derive(Debug)]
pub enum ParsedIpcRequest {
    FetchEnv(IpcFetchEnvRequest),
    SandboxCreate(SandboxCreateRequest),
    SandboxRegisterPids(SandboxRegisterPidsRequest),
    SandboxRevoke(SandboxRevokeRequest),
    Unknown {
        request_id: String,
        msg_type: String,
    },
}

/// Parse IPC line — v3 env fetch (no `type`) or v4 sandbox messages.
pub fn parse_incoming(line: &str) -> Result<ParsedIpcRequest, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(line.trim())?;
    if let Some(t) = value.get("type").and_then(|v| v.as_str()) {
        match t {
            "sandbox_create" => {
                let req: SandboxCreateRequest = serde_json::from_value(value)?;
                Ok(ParsedIpcRequest::SandboxCreate(req))
            }
            "sandbox_register_pids" => {
                let req: SandboxRegisterPidsRequest = serde_json::from_value(value)?;
                Ok(ParsedIpcRequest::SandboxRegisterPids(req))
            }
            "sandbox_revoke" => {
                let req: SandboxRevokeRequest = serde_json::from_value(value)?;
                Ok(ParsedIpcRequest::SandboxRevoke(req))
            }
            other => Ok(ParsedIpcRequest::Unknown {
                request_id: value
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                msg_type: other.to_string(),
            }),
        }
    } else {
        if value.get("command_preview").is_some() {
            return Err(serde::de::Error::custom(
                "sandbox IPC request missing \"type\":\"sandbox_create\"",
            ));
        }
        let req: IpcFetchEnvRequest = serde_json::from_value(value)?;
        Ok(ParsedIpcRequest::FetchEnv(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v3_fetch_env() {
        let line = r#"{"request_id":"r1","bucket_id":"b1","client_token":"tok"}"#;
        let parsed = parse_incoming(line).unwrap();
        assert!(matches!(parsed, ParsedIpcRequest::FetchEnv(_)));
    }

    #[test]
    fn sandbox_create_response_roundtrip() {
        let resp = IpcResponse::Ok {
            request_id: "r1".into(),
            env: HashMap::new(),
            proxy: None,
            session_id: Some("sess_abc".into()),
            proxy_port: Some(9000),
            expires_at: Some("2026-01-01T00:00:00Z".into()),
            ca_bundle_path: Some("/tmp/ca.pem".into()),
        };
        let line = resp.to_line();
        assert!(line.contains("session_id"));
        let parsed: IpcResponse = serde_json::from_str(&line).unwrap();
        match parsed {
            IpcResponse::Ok {
                session_id: Some(id),
                proxy_port: Some(9000),
                ..
            } => assert_eq!(id, "sess_abc"),
            _ => panic!("expected sandbox ok response"),
        }
    }

    #[test]
    fn sandbox_create_request_includes_type() {
        let req = SandboxCreateRequest {
            msg_type: "sandbox_create".into(),
            request_id: "r1".into(),
            bucket_id: "b1".into(),
            client_token: "tok".into(),
            cwd: None,
            command_preview: Some("cmd /c echo".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"sandbox_create""#));
    }

    #[test]
    fn parses_sandbox_create() {
        let line = r#"{"type":"sandbox_create","request_id":"r1","bucket_id":"b1","client_token":"tok"}"#;
        assert!(matches!(
            parse_incoming(line).unwrap(),
            ParsedIpcRequest::SandboxCreate(_)
        ));
    }
}
