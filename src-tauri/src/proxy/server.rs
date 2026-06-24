use std::io;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Instant;

use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderName, HeaderValue, CONTENT_LENGTH};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_rustls::HttpsConnectorBuilder;
use rusqlite::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsAcceptor;

use crate::infra::db::audit;
use crate::infra::db::bucket_mappings;
use crate::error::{AppError, AppResult};
use crate::ipc::VerifiedClient;
use crate::proxy::auth::{authenticate_proxy_headers, verify_grant};
use crate::proxy::ca::{self, upstream_root_store};
use crate::proxy::peer_tcp::peer_pid_from_stream;
use crate::proxy::rewrite::{rewrite_body, rewrite_headers};
use crate::proxy::transparent;
use crate::state::AppState;
use protocol::capture_log;
use protocol::relay_frame;
use tauri::Manager;

const TLS_HANDSHAKE: u8 = 0x16;

/// First-byte routing for shared bucket listener (library vs transparent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingRoute {
    TransparentTls,
    MaybeHttp,
    NotImplemented,
}

pub fn route_incoming_first_byte(b: u8) -> IncomingRoute {
    if b == TLS_HANDSHAKE {
        IncomingRoute::TransparentTls
    } else if b == b'C' || b == b'c' {
        IncomingRoute::MaybeHttp
    } else {
        IncomingRoute::NotImplemented
    }
}

pub struct BucketProxyContext {
    pub bucket_id: String,
    pub app: tauri::AppHandle,
}

#[derive(Clone)]
pub struct MitmAuditMeta {
    pub session_id: Option<String>,
    pub capture_mode: &'static str,
    pub pid: Option<u32>,
}

impl Default for MitmAuditMeta {
    fn default() -> Self {
        Self {
            session_id: None,
            capture_mode: "explicit",
            pid: None,
        }
    }
}

pub struct ProxyServerHandle {
    shutdown: watch::Sender<bool>,
    join: Option<JoinHandle<()>>,
}

impl ProxyServerHandle {
    pub fn stop(mut self) {
        let _ = self.shutdown.send(true);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

type UpstreamClient = Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<bytes::Bytes>>;

fn upstream_client() -> &'static UpstreamClient {
    static CLIENT: OnceLock<UpstreamClient> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let roots = upstream_root_store();
        let tls = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        let https = HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http()
            .enable_http1()
            .build();
        Client::builder(TokioExecutor::new()).build(https)
    })
}

/// Start a per-bucket proxy on a background thread with its own Tokio runtime.
pub fn start_bucket_proxy(
    app: tauri::AppHandle,
    bucket_id: String,
    port: u16,
) -> AppResult<ProxyServerHandle> {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let ctx = Arc::new(BucketProxyContext {
        bucket_id: bucket_id.clone(),
        app,
    });

    let thread_name = format!("argus-proxy-{port}");
    let join = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("bucket proxy {bucket_id}: failed to create runtime: {e}");
                    return;
                }
            };
            if let Err(e) = rt.block_on(run_listener(port, ctx, shutdown_rx)) {
                eprintln!("bucket proxy {bucket_id} on {port} stopped: {e}");
            }
        })
        .map_err(|e| AppError::message("PROXY_ERROR", e.to_string()))?;

    Ok(ProxyServerHandle {
        shutdown: shutdown_tx,
        join: Some(join),
    })
}

async fn run_listener(
    port: u16,
    ctx: Arc<BucketProxyContext>,
    mut shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            accept = listener.accept() => {
                let (stream, _) = accept?;
                let c = ctx.clone();
                tokio::spawn(async move {
                    let _ = handle_client(stream, c).await;
                });
            }
        }
    }
    Ok(())
}

async fn handle_client(mut stream: TcpStream, ctx: Arc<BucketProxyContext>) -> io::Result<()> {
    let peer = stream.peer_addr().ok();
    let peer_loopback = peer.map(|a| a.ip().is_loopback()).unwrap_or(false);

    let mut first = [0u8; 1];
    let n = stream.read(&mut first).await?;
    if n == 0 {
        return Ok(());
    }

    capture_log::log(
        "proxy",
        format!(
            "accepted from {:?} first_byte=0x{:02x}",
            peer, first[0]
        ),
    );

    let relay_pid = if peer_loopback && first[0] == relay_frame::first_byte() {
        let mut rest = [0u8; relay_frame::HEADER_LEN - 1];
        if stream.read_exact(&mut rest).await.is_err() {
            capture_log::log("proxy", "relay header truncated");
            return Ok(());
        }
        let mut hdr = [0u8; relay_frame::HEADER_LEN];
        hdr[0] = first[0];
        hdr[1..].copy_from_slice(&rest);
        match relay_frame::decode(&hdr) {
            Some(pid) => {
                let mut tls_first = [0u8; 1];
                if stream.read_exact(&mut tls_first).await.is_err() {
                    capture_log::log("proxy", "TLS byte missing after relay header");
                    return Ok(());
                }
                first = tls_first;
                capture_log::log("proxy", format!("relay header ok pid={pid}"));
                Some(pid)
            }
            None => {
                capture_log::log("proxy", "invalid relay header magic");
                stream
                    .write_all(b"HTTP/1.1 501 Not Implemented\r\nContent-Length: 0\r\n\r\n")
                    .await?;
                return Ok(());
            }
        }
    } else {
        None
    };

    if first[0] == TLS_HANDSHAKE {
        let record = read_tls_first_record(&mut stream, first[0]).await?;
        capture_log::log(
            "proxy",
            format!(
                "transparent TLS record {} B relay_pid={relay_pid:?}",
                record.len()
            ),
        );
        return transparent::handle_transparent(record, stream, ctx, relay_pid).await;
    }

    if first[0] == b'C' || first[0] == b'c' {
        let headers = read_http_headers_prefixed(&mut stream, vec![first[0]]).await?;
        if headers.is_empty() {
            return Ok(());
        }
        let method = headers
            .first()
            .map(|l| l.split_whitespace().next().unwrap_or(""))
            .unwrap_or("");
        if method.eq_ignore_ascii_case("CONNECT") {
            return handle_connect(stream, &headers, ctx).await;
        }
        return handle_plain_http(stream, &headers, ctx).await;
    }

    stream
        .write_all(b"HTTP/1.1 501 Not Implemented\r\nContent-Length: 0\r\n\r\n")
        .await
}

fn tls_record_len16(b: &[u8]) -> usize {
    u16::from_be_bytes([b[0], b[1]]) as usize
}

async fn read_tls_first_record(stream: &mut TcpStream, first: u8) -> io::Result<Vec<u8>> {
    let mut buf = vec![first];
    while buf.len() < 5 {
        let mut c = [0u8; 1];
        stream.read_exact(&mut c).await?;
        buf.push(c[0]);
    }
    let record_len = tls_record_len16(&buf[3..5]);
    let total = 5 + record_len;
    while buf.len() < total && buf.len() < 64 * 1024 {
        let mut chunk = vec![0u8; (total - buf.len()).min(4096)];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}

async fn read_http_headers_prefixed(
    stream: &mut TcpStream,
    mut buf: Vec<u8>,
) -> io::Result<Vec<String>> {
    loop {
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > 64 * 1024 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "headers too large"));
        }
        let mut c = [0u8; 1];
        stream.read_exact(&mut c).await?;
        buf.push(c[0]);
    }
    let text = String::from_utf8_lossy(&buf);
    Ok(text.lines().map(|l| l.to_string()).collect())
}

fn connect_target(first_line: &str) -> Option<String> {
    let parts: Vec<_> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        return Some(parts[1].to_string());
    }
    None
}

fn parse_header_pairs(lines: &[String]) -> Vec<(String, String)> {
    lines
        .iter()
        .skip(1)
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

async fn handle_connect(
    mut stream: TcpStream,
    lines: &[String],
    ctx: Arc<BucketProxyContext>,
) -> io::Result<()> {
    let first = lines.first().cloned().unwrap_or_default();
    let target = connect_target(&first).unwrap_or_default();
    let host = target.split(':').next().unwrap_or(&target).to_string();

    let header_pairs = parse_header_pairs(lines);
    let started = Instant::now();
    let pid = peer_pid_from_stream(&stream).ok();

    let gate = with_db(&ctx, |conn, vk| {
        let auth = authenticate_proxy_headers(conn, &header_pairs)?;
        if auth.bucket_id != ctx.bucket_id {
            return Ok((false, false, vec![]));
        }
        if !bucket_mappings::bucket_allows_proxy_host(conn, &ctx.bucket_id, &host)? {
            let _ = audit::proxy_host_denied(conn, &ctx.bucket_id, &host, pid.unwrap_or(0));
            return Ok((false, false, vec![]));
        }
        let peer = pid.and_then(|p| VerifiedClient::from_pid(p, None).ok());
        let grant_ok = verify_grant(conn, &auth, peer.as_ref())?;
        if !grant_ok {
            let _ = audit::proxy_grant_denied(conn, &ctx.bucket_id, pid.unwrap_or(0));
            return Ok((true, false, vec![]));
        }
        let entries = bucket_mappings::list_proxy_rewrite_entries(conn, &ctx.bucket_id, vk, &host)?;
        Ok((true, true, entries))
    });

    let (host_allowed, grant_ok, entries) = match gate {
        Ok(v) => v,
        Err(()) => {
            stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await?;
            return Ok(());
        }
    };

    if !host_allowed {
        stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            .await?;
        return Ok(());
    }
    if !grant_ok {
        stream
            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
            .await?;
        return Ok(());
    }

    stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    let server_cfg = match ca::server_config_for_host(&host) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    let acceptor = TlsAcceptor::from(server_cfg);
    let mut tls_client = match acceptor.accept(stream).await {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let io = TokioIo::new(&mut tls_client);
    let entries = Arc::new(entries);
    let host_c = host.clone();
    let ctx_c = ctx.clone();
    let audit_meta = MitmAuditMeta {
        session_id: None,
        capture_mode: "explicit",
        pid,
    };
    let service = service_fn(move |req: Request<Incoming>| {
        let entries = entries.clone();
        let host = host_c.clone();
        let ctx = ctx_c.clone();
        let started = started;
        let audit_meta = audit_meta.clone();
        async move {
            Ok::<_, hyper::Error>(
                handle_mitm_request(req, entries, host, ctx, started, audit_meta).await,
            )
        }
    });

    if http1::Builder::new()
        .serve_connection(io, service)
        .await
        .is_err()
    {}

    Ok(())
}

pub(crate) fn proxy_error_response(status: StatusCode) -> Response<Full<bytes::Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(bytes::Bytes::new()))
        .unwrap()
}

fn skip_request_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "proxy-connection"
            | "proxy-authorization"
            | "keep-alive"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn skip_response_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "transfer-encoding"
            | "keep-alive"
            | "proxy-authenticate"
            | "te"
            | "trailer"
            | "upgrade"
    )
}

fn build_upstream_request(
    method: hyper::Method,
    upstream_uri: &str,
    headers: &hyper::HeaderMap,
    body: bytes::Bytes,
) -> Result<Request<Full<bytes::Bytes>>, ()> {
    let uri: hyper::Uri = upstream_uri.parse().map_err(|_| ())?;
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers.iter() {
        if skip_request_header(name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    builder.body(Full::new(body)).map_err(|_| ())
}

pub(crate) fn build_mitm_response(
    status: StatusCode,
    headers: &hyper::HeaderMap,
    body: bytes::Bytes,
) -> Response<Full<bytes::Bytes>> {
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        if skip_response_header(name) {
            continue;
        }
        builder = builder.header(name, value);
    }
    if !body.is_empty() {
        if let Ok(len) = HeaderValue::from_str(&body.len().to_string()) {
            builder = builder.header(CONTENT_LENGTH, len);
        }
    }
    builder
        .body(Full::new(body))
        .unwrap_or_else(|_| proxy_error_response(StatusCode::BAD_GATEWAY))
}

pub async fn handle_mitm_request(
    mut req: Request<Incoming>,
    entries: Arc<Vec<bucket_mappings::ProxyRewriteEntry>>,
    host: String,
    ctx: Arc<BucketProxyContext>,
    started: Instant,
    audit_meta: MitmAuditMeta,
) -> Response<Full<bytes::Bytes>> {
    let method = req.method().clone();
    let method_str = method.as_str().to_string();
    let path = req.uri().path().to_string();
    let uri = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/")
        .to_string();

    let used_label = rewrite_headers(req.headers_mut(), &entries);
    let client_headers = req.headers().clone();

    let upstream_uri = format!("https://{host}{uri}");

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => bytes::Bytes::new(),
    };
    let body_bytes = rewrite_body(&body_bytes, &entries);

    let upstream_req = match build_upstream_request(
        method,
        &upstream_uri,
        &client_headers,
        body_bytes,
    ) {
        Ok(r) => r,
        Err(()) => return proxy_error_response(StatusCode::BAD_GATEWAY),
    };

    let upstream_resp = match upstream_client().request(upstream_req).await {
        Ok(r) => r,
        Err(_) => return proxy_error_response(StatusCode::BAD_GATEWAY),
    };

    let status = upstream_resp.status();
    let upstream_headers = upstream_resp.headers().clone();
    let resp_body = match upstream_resp.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return proxy_error_response(StatusCode::BAD_GATEWAY),
    };

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let pid = audit_meta.pid.unwrap_or(0);

    let _ = with_db(&ctx, |conn, _| {
        audit::proxy_request(
            conn,
            &ctx.bucket_id,
            &host,
            &path,
            &method_str,
            used_label.as_deref(),
            status.as_u16(),
            elapsed_ms,
            pid,
            audit_meta.session_id.as_deref(),
            audit_meta.capture_mode,
        )
    });

    build_mitm_response(status, &upstream_headers, resp_body)
}

async fn handle_plain_http(
    mut stream: TcpStream,
    lines: &[String],
    ctx: Arc<BucketProxyContext>,
) -> io::Result<()> {
    let header_pairs = parse_header_pairs(lines);
    let _ = with_db(&ctx, |conn, _| authenticate_proxy_headers(conn, &header_pairs));
    stream
        .write_all(b"HTTP/1.1 501 Not Implemented\r\nContent-Length: 0\r\n\r\n")
        .await
}

pub fn with_db<T, F>(ctx: &BucketProxyContext, f: F) -> Result<T, ()>
where
    F: FnOnce(&Connection, &[u8; 32]) -> AppResult<T>,
{
    let state = ctx.app.state::<AppState>();
    let inner = state.0.lock().map_err(|_| ())?;
    let pool = inner.db.as_ref().ok_or(())?;
    let vk = inner.value_key().ok_or(())?;
    let conn = pool.lock().map_err(|_| ())?;
    f(&conn, &vk).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::relay_frame;

    #[test]
    fn tls_record_len_parsing() {
        assert_eq!(tls_record_len16(&[0x01, 0x00]), 256);
        assert_eq!(tls_record_len16(&[0x01, 0x7c]), 380);
    }

    #[test]
    fn connect_target_parsing() {
        assert_eq!(
            connect_target("CONNECT api.example.com:443 HTTP/1.1"),
            Some("api.example.com:443".to_string())
        );
    }

    #[test]
    fn relay_header_peel_before_tls() {
        let pid = 35_096u32;
        let hdr = relay_frame::encode(pid);
        assert_eq!(hdr[0], relay_frame::first_byte());
        assert_eq!(relay_frame::decode(&hdr), Some(pid));
        let mut combined = hdr.to_vec();
        combined.push(0x16);
        assert_eq!(
            relay_frame::decode(&combined[..relay_frame::HEADER_LEN]),
            Some(pid)
        );
        assert_eq!(combined[relay_frame::HEADER_LEN], 0x16);
    }

    #[test]
    fn first_byte_routes_connect_and_tls() {
        assert_eq!(
            route_incoming_first_byte(0x16),
            IncomingRoute::TransparentTls
        );
        assert_eq!(route_incoming_first_byte(b'C'), IncomingRoute::MaybeHttp);
        assert_eq!(route_incoming_first_byte(b'G'), IncomingRoute::NotImplemented);
    }
}
