//! Local IPC client for Argus desktop (Unix socket / Windows named pipe).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

use protocol::{
    ipc_pipe_name, IpcFetchEnvRequest, IpcResponse, ProxyConfigPayload, SandboxListRequest,
    SandboxSessionInfo, SandboxCreateRequest, SandboxRegisterPidsRequest, SandboxRevokeRequest,
};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(130);

#[derive(Debug, Error)]
pub enum IpcClientError {
    #[error("Start Argus and sign in. IPC socket not found at {path}. {hint}")]
    SocketNotFound { path: String, hint: String },
    #[error("Argus returned an error ({code}): {message}")]
    Api { code: String, message: String },
    #[error("Argus denied the request ({code}): {message}")]
    Denied { code: String, message: String },
    #[error("Argus vault is locked: {message}")]
    Locked { message: String },
    #[error("Invalid IPC response: {0}")]
    InvalidResponse(String),
    #[error("IPC I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Timed out after {0:?} waiting for Argus IPC response")]
    Timeout(Duration),
}

#[derive(Debug, Clone)]
pub struct FetchEnvResult {
    pub env: HashMap<String, String>,
    pub proxy: Option<ProxyConfigPayload>,
}

#[derive(Debug, Clone)]
pub struct SandboxCreateResult {
    pub session_id: String,
    pub proxy_port: u16,
    pub expires_at: String,
    pub env: HashMap<String, String>,
    pub ca_bundle_path: String,
    pub relay_secret: String,
    pub no_proxy: bool,
}

#[derive(Debug, Clone)]
pub struct SandboxSessionListItem {
    pub session_id: String,
    pub bucket_id: String,
    pub command_preview: Option<String>,
    pub expires_at: String,
    pub pids: Vec<u32>,
}

/// Human-readable IPC endpoint (Unix socket path or Windows pipe name).
pub fn ipc_endpoint() -> String {
    if cfg!(windows) {
        ipc_pipe_name()
    } else {
        socket_path().display().to_string()
    }
}

pub fn socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".argus")
        .join("argus.sock")
}

fn connection_hint() -> String {
    if cfg!(windows) {
        format!(
            "Is Argus signed in and running? The named pipe {} must exist.",
            ipc_pipe_name()
        )
    } else {
        format!(
            "Is Argus signed in and running? Expected Unix socket at {}.",
            socket_path().display()
        )
    }
}

fn exchange_line(payload: &str, timeout: Duration) -> Result<String, IpcClientError> {
    #[cfg(windows)]
    {
        return exchange_windows(payload, timeout);
    }
    #[cfg(unix)]
    {
        return exchange_unix(payload, timeout);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (payload, timeout);
        Err(IpcClientError::InvalidResponse("unsupported platform".into()))
    }
}

#[cfg(unix)]
fn exchange_unix(payload: &str, timeout: Duration) -> Result<String, IpcClientError> {
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;

    let path = socket_path();
    if !path.exists() {
        return Err(IpcClientError::SocketNotFound {
            path: path.display().to_string(),
            hint: connection_hint(),
        });
    }
    let mut stream = UnixStream::connect(&path)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(payload.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let _ = reader.into_inner().shutdown(Shutdown::Both);
    Ok(line.trim().to_string())
}

#[cfg(windows)]
fn exchange_windows(payload: &str, timeout: Duration) -> Result<String, IpcClientError> {
    use std::fs::OpenOptions;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    let payload = payload.to_string();
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = (|| -> Result<String, std::io::Error> {
            let pipe_name = ipc_pipe_name();
            let mut pipe = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&pipe_name)?;
            pipe.write_all(payload.as_bytes())?;
            pipe.write_all(b"\n")?;
            let mut reader = BufReader::new(pipe);
            let mut line = String::new();
            reader.read_line(&mut line)?;
            Ok(line.trim().to_string())
        })();
        let _ = tx.send(result);
    });

    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(IpcClientError::Timeout(timeout));
        }
        match rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
            Ok(Ok(line)) => return Ok(line),
            Ok(Err(e)) => return Err(IpcClientError::Io(e)),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(IpcClientError::InvalidResponse(
                    "IPC worker exited unexpectedly".into(),
                ));
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn exchange_unix(_payload: &str, _timeout: Duration) -> Result<String, IpcClientError> {
    Err(IpcClientError::InvalidResponse("unsupported platform".into()))
}

fn parse_response(raw: &str) -> Result<IpcResponse, IpcClientError> {
    serde_json::from_str(raw).map_err(|e| IpcClientError::InvalidResponse(e.to_string()))
}

fn map_response(resp: IpcResponse) -> Result<IpcResponse, IpcClientError> {
    match resp {
        IpcResponse::Ok { .. } => Ok(resp),
        IpcResponse::Denied {
            request_id: _,
            code,
            message,
        } => Err(IpcClientError::Denied { code, message }),
        IpcResponse::Locked {
            request_id: _,
            message,
        } => Err(IpcClientError::Locked { message }),
        IpcResponse::Error {
            request_id: _,
            code,
            message,
        } => Err(IpcClientError::Api { code, message }),
    }
}

fn send_json(value: &impl serde::Serialize, timeout: Duration) -> Result<IpcResponse, IpcClientError> {
    let line = serde_json::to_string(value)
        .map_err(|e| IpcClientError::InvalidResponse(e.to_string()))?;
    let raw = exchange_line(&line, timeout)?;
    map_response(parse_response(&raw)?)
}

fn sandbox_response_hint(raw: &str) -> String {
    if raw.contains("\"session_id\"") || raw.contains("\"sessionId\"") {
        return "missing session_id in parsed sandbox response".into();
    }
    "Argus returned a library-mode env response without a sandbox session. \
     Restart the desktop from source (`pnpm tauri dev`) — an older installed build \
     may not support `argus run` yet, or approve the CLI access request in Argus."
        .into()
}

pub fn fetch_bucket_env(
    bucket_id: &str,
    client_token: &str,
    cwd: Option<&str>,
    timeout: Duration,
) -> Result<FetchEnvResult, IpcClientError> {
    let req = IpcFetchEnvRequest {
        request_id: Uuid::new_v4().to_string(),
        bucket_id: bucket_id.to_string(),
        client_token: client_token.to_string(),
        cwd: cwd.map(str::to_string),
    };
    match send_json(&req, timeout)? {
        IpcResponse::Ok { env, proxy, .. } => Ok(FetchEnvResult { env, proxy }),
        _ => Err(IpcClientError::InvalidResponse("unexpected response".into())),
    }
}

pub fn sandbox_create(
    bucket_id: &str,
    client_token: &str,
    cwd: Option<&str>,
    command_preview: Option<&str>,
    no_proxy: bool,
    timeout: Duration,
) -> Result<SandboxCreateResult, IpcClientError> {
    let req = SandboxCreateRequest {
        msg_type: "sandbox_create".into(),
        request_id: Uuid::new_v4().to_string(),
        bucket_id: bucket_id.to_string(),
        client_token: client_token.to_string(),
        cwd: cwd.map(str::to_string),
        command_preview: command_preview.map(str::to_string),
        no_proxy,
    };
    let line = serde_json::to_string(&req)
        .map_err(|e| IpcClientError::InvalidResponse(e.to_string()))?;
    let raw = exchange_line(&line, timeout)?;
    let resp = map_response(parse_response(&raw)?)?;
    match resp {
        IpcResponse::Ok {
            session_id,
            proxy_port,
            expires_at,
            env,
            ca_bundle_path,
            relay_secret,
            ..
        } => {
            let session_id = session_id.ok_or_else(|| {
                IpcClientError::InvalidResponse(sandbox_response_hint(&raw))
            })?;
            if no_proxy {
                Ok(SandboxCreateResult {
                    session_id,
                    proxy_port: proxy_port.unwrap_or(0),
                    expires_at: expires_at.unwrap_or_default(),
                    env,
                    ca_bundle_path: ca_bundle_path.unwrap_or_default(),
                    relay_secret: relay_secret.unwrap_or_default(),
                    no_proxy: true,
                })
            } else {
                Ok(SandboxCreateResult {
                    session_id,
                    proxy_port: proxy_port.ok_or_else(|| {
                        IpcClientError::InvalidResponse(
                            "missing proxy_port — enable Argus Proxy on this bucket in the app"
                                .into(),
                        )
                    })?,
                    expires_at: expires_at.unwrap_or_default(),
                    env,
                    ca_bundle_path: ca_bundle_path.unwrap_or_default(),
                    relay_secret: relay_secret.unwrap_or_default(),
                    no_proxy: false,
                })
            }
        }
        _ => Err(IpcClientError::InvalidResponse("unexpected response".into())),
    }
}

pub fn sandbox_register_pids(
    session_id: &str,
    pids: &[u32],
    timeout: Duration,
) -> Result<(), IpcClientError> {
    let req = SandboxRegisterPidsRequest {
        msg_type: "sandbox_register_pids".into(),
        request_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        pids: pids.to_vec(),
    };
    match send_json(&req, timeout)? {
        IpcResponse::Ok { .. } => Ok(()),
        _ => Err(IpcClientError::InvalidResponse("unexpected response".into())),
    }
}

pub fn sandbox_revoke(session_id: &str, timeout: Duration) -> Result<(), IpcClientError> {
    let req = SandboxRevokeRequest {
        msg_type: "sandbox_revoke".into(),
        request_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
    };
    match send_json(&req, timeout)? {
        IpcResponse::Ok { .. } => Ok(()),
        _ => Err(IpcClientError::InvalidResponse("unexpected response".into())),
    }
}

pub fn sandbox_list(timeout: Duration) -> Result<Vec<SandboxSessionListItem>, IpcClientError> {
    let req = SandboxListRequest {
        msg_type: "sandbox_list".into(),
        request_id: Uuid::new_v4().to_string(),
    };
    match send_json(&req, timeout)? {
        IpcResponse::Ok { sessions: Some(list), .. } => Ok(list
            .into_iter()
            .map(|s: SandboxSessionInfo| SandboxSessionListItem {
                session_id: s.session_id,
                bucket_id: s.bucket_id,
                command_preview: s.command_preview,
                expires_at: s.expires_at,
                pids: s.pids,
            })
            .collect()),
        IpcResponse::Ok { .. } => Ok(vec![]),
        _ => Err(IpcClientError::InvalidResponse("unexpected response".into())),
    }
}

pub fn ping(timeout: Duration) -> Result<bool, IpcClientError> {
    #[cfg(unix)]
    {
        if !socket_path().exists() {
            return Ok(false);
        }
    }

    // Lightweight IPC probe: invalid bucket id should still get a structured error/locked
    // response when the desktop is signed in and listening.
    let req = IpcFetchEnvRequest {
        request_id: Uuid::new_v4().to_string(),
        bucket_id: "__argus_cli_ping__".into(),
        client_token: "ping".into(),
        cwd: None,
    };
    match send_json(&req, timeout) {
        Ok(_) => Ok(true),
        Err(IpcClientError::Api { .. })
        | Err(IpcClientError::Denied { .. })
        | Err(IpcClientError::Locked { .. }) => Ok(true),
        Err(IpcClientError::SocketNotFound { .. }) => Ok(false),
        Err(IpcClientError::Io(_)) => Ok(false),
        Err(IpcClientError::Timeout(_)) => Ok(false),
        Err(IpcClientError::InvalidResponse(_)) => Ok(false),
    }
}
