//! Re-export shared IPC protocol types from `protocol`.

pub use protocol::*;

// Legacy alias used by desktop IPC server.
pub type IpcRequest = IpcFetchEnvRequest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sandbox_revoke() {
        let line = r#"{"type":"sandbox_revoke","request_id":"r1","session_id":"sess_abc"}"#;
        let parsed = parse_incoming(line).unwrap();
        match parsed {
            ParsedIpcRequest::SandboxRevoke(req) => assert_eq!(req.session_id, "sess_abc"),
            _ => panic!("expected sandbox_revoke"),
        }
    }
}
