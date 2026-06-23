use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;

use crate::infra::db::audit;
use crate::infra::db::bucket_mappings;
use crate::infra::db::sandbox_sessions;
use crate::proxy::ca;
use crate::proxy::peer_tcp::peer_pid_from_stream;
use crate::proxy::server::{
    handle_mitm_request, with_db, BucketProxyContext, MitmAuditMeta,
};
use crate::proxy::tls_sni::sni_from_client_hello;
use protocol::capture_log;

const TLS_HANDSHAKE: u8 = 0x16;

/// TCP stream with bytes already read for protocol sniffing prepended.
pub struct PrefixedStream {
    prefix: Vec<u8>,
    pos: usize,
    inner: TcpStream,
}

impl PrefixedStream {
    pub fn new(prefix: Vec<u8>, inner: TcpStream) -> Self {
        Self {
            prefix,
            pos: 0,
            inner,
        }
    }

    pub fn into_inner(self) -> TcpStream {
        self.inner
    }
}

impl AsyncRead for PrefixedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.pos < self.prefix.len() {
            let remaining = &self.prefix[self.pos..];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            self.pos += to_copy;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PrefixedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Result of sandbox session + host gate before MITM.
pub struct TransparentGateOk {
    pub session_id: String,
    pub host: String,
    pub entries: Vec<bucket_mappings::ProxyRewriteEntry>,
    pub pid: u32,
}

pub enum TransparentGateResult {
    Ok(TransparentGateOk),
    Deny { pid: Option<u32>, reason: &'static str },
}

/// Evaluate transparent-path authorization (session, SNI, allowlist).
pub fn evaluate_transparent_gate(
    conn: &rusqlite::Connection,
    vk: &[u8; 32],
    bucket_id: &str,
    pid: Option<u32>,
    prefix: &[u8],
) -> TransparentGateResult {
    let Some(pid) = pid else {
        return TransparentGateResult::Deny {
            pid: None,
            reason: "no_pid",
        };
    };
    let session = match sandbox_sessions::lookup_active_session_by_pid(conn, pid) {
        Ok(Some(s)) if s.bucket_id == bucket_id => s,
        Ok(_) | Err(_) => {
            return TransparentGateResult::Deny {
                pid: Some(pid),
                reason: "session_or_sni",
            };
        }
    };
    let host = match sni_from_client_hello(prefix) {
        Ok(Some(h)) => h,
        _ => {
            return TransparentGateResult::Deny {
                pid: Some(pid),
                reason: "session_or_sni",
            };
        }
    };
    if !bucket_mappings::bucket_allows_proxy_host(conn, bucket_id, &host).unwrap_or(false) {
        let _ = audit::proxy_host_denied(conn, bucket_id, &host, pid);
        return TransparentGateResult::Deny {
            pid: Some(pid),
            reason: "host_denied",
        };
    }
    let entries = match bucket_mappings::list_proxy_rewrite_entries(conn, bucket_id, vk, &host) {
        Ok(e) => e,
        Err(_) => {
            return TransparentGateResult::Deny {
                pid: Some(pid),
                reason: "session_or_sni",
            };
        }
    };
    TransparentGateResult::Ok(TransparentGateOk {
        session_id: session.id,
        host,
        entries,
        pid,
    })
}

pub async fn handle_transparent(
    prefix: Vec<u8>,
    stream: TcpStream,
    ctx: Arc<BucketProxyContext>,
    relay_pid: Option<u32>,
) -> io::Result<()> {
    if prefix.first().copied() != Some(TLS_HANDSHAKE) {
        return Ok(());
    }

    let pid = relay_pid.or_else(|| peer_pid_from_stream(&stream).ok());
    capture_log::log(
        "gate",
        format!(
            "relay_pid={relay_pid:?} peer_pid={} bucket={}",
            pid.map(|p| p.to_string())
                .unwrap_or_else(|| "none".into()),
            ctx.bucket_id
        ),
    );
    let started = Instant::now();

    let gate_result = match with_db(&ctx, |conn, vk| {
        Ok(evaluate_transparent_gate(
            conn,
            vk,
            &ctx.bucket_id,
            pid,
            &prefix,
        ))
    }) {
        Ok(r) => r,
        Err(()) => return Ok(()),
    };

    let (session_id, host, entries, pid) = match gate_result {
        TransparentGateResult::Ok(ok) => {
            capture_log::log(
                "gate",
                format!(
                    "ALLOW session={} host={} pid={} rewrite_entries={}",
                    ok.session_id,
                    ok.host,
                    ok.pid,
                    ok.entries.len()
                ),
            );
            (ok.session_id, ok.host, ok.entries, ok.pid)
        }
        TransparentGateResult::Deny { pid: gate_pid, reason } => {
            let host = sni_from_client_hello(&prefix)
                .ok()
                .flatten()
                .unwrap_or_else(|| "unknown".to_string());
            let _ = with_db(&ctx, |conn, _| {
                if reason == "host_denied" {
                    if let Some(p) = gate_pid {
                        audit::proxy_host_denied(conn, &ctx.bucket_id, &host, p)?;
                    }
                } else if let Some(p) = gate_pid {
                    audit::proxy_grant_denied(conn, &ctx.bucket_id, p)?;
                    audit::sandbox_transparent_denied(conn, &ctx.bucket_id, p, reason)?;
                } else {
                    audit::sandbox_transparent_denied(conn, &ctx.bucket_id, 0, reason)?;
                }
                Ok(())
            });
            capture_log::log(
                "gate",
                format!("DENY reason={reason} pid={gate_pid:?} host={host}"),
            );
            eprintln!(
                "argus transparent gate denied (bucket={}, reason={}, pid={gate_pid:?}, host={host})",
                ctx.bucket_id, reason
            );
            return Ok(());
        }
    };

    let server_cfg = match ca::server_config_for_host(&host) {
        Ok(c) => c,
        Err(e) => {
            capture_log::log("gate", format!("MITM CA issue for {host}: {e}"));
            return Ok(());
        }
    };
    let acceptor = TlsAcceptor::from(server_cfg);
    let prefixed = PrefixedStream::new(prefix, stream);
    let mut tls_client = match acceptor.accept(prefixed).await {
        Ok(s) => {
            capture_log::log("gate", format!("TLS accept ok host={host}"));
            s
        }
        Err(e) => {
            capture_log::log("gate", format!("TLS accept failed host={host}: {e}"));
            return Ok(());
        }
    };

    let io = TokioIo::new(&mut tls_client);
    let entries = Arc::new(entries);
    let host_c = host.clone();
    let ctx_c = ctx.clone();
    let audit_meta = MitmAuditMeta {
        session_id: Some(session_id),
        capture_mode: "transparent",
        pid: Some(pid),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::db::meta::run_migrations;
    use chrono::Utc;
    use rusqlite::{params, Connection};
    use uuid::Uuid;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn seed_bucket(conn: &Connection, allowed_hosts: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_buckets (id, name, client_token_hash, client_token_enc,
             access_ttl_minutes, is_tray_active, proxy_enabled, proxy_port, allowed_hosts,
             created_at, updated_at)
             VALUES (?1, 'test', x'00', x'00', 60, 1, 1, 9001, ?2, ?3, ?3)",
            params![id, allowed_hosts, now],
        )
        .unwrap();
        id
    }

    fn minimal_client_hello_with_sni(host: &str) -> Vec<u8> {
        // TLS record header + minimal ClientHello with SNI extension for `host`.
        let mut hello = vec![
            0x03, 0x03, // version
        ];
        hello.extend_from_slice(&[0u8; 32]); // random
        hello.push(0); // session id len
        hello.extend_from_slice(&[0, 2, 0x00, 0x2f]); // cipher suites
        hello.push(1); // compression
        hello.push(0); // comp method
        let host_bytes = host.as_bytes();
        let sni_list_len = 3 + host_bytes.len();
        let ext_len = 2 + sni_list_len;
        let mut exts = Vec::new();
        exts.extend_from_slice(&[0x00, 0x00]); // server_name ext type
        exts.extend_from_slice(&(ext_len as u16).to_be_bytes());
        exts.extend_from_slice(&(sni_list_len as u16).to_be_bytes());
        exts.push(0); // host_name type
        exts.extend_from_slice(&(host_bytes.len() as u16).to_be_bytes());
        exts.extend_from_slice(host_bytes);
        hello.extend_from_slice(&(exts.len() as u16).to_be_bytes());
        hello.extend_from_slice(&exts);
        let hello_len = hello.len();
        let mut body = vec![0x01]; // ClientHello
        body.extend_from_slice(&(hello_len as u32).to_be_bytes()[1..]); // 3-byte len
        body.extend_from_slice(&hello);
        let record_len = body.len();
        let mut record = vec![0x16, 0x03, 0x01];
        record.extend_from_slice(&(record_len as u16).to_be_bytes());
        record.extend_from_slice(&body);
        record
    }

    #[test]
    fn gate_allows_registered_pid_and_host() {
        let conn = mem_conn();
        let bucket_id = seed_bucket(&conn, r#"["api.example.com"]"#);
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            "grant-1",
            "fp",
            None,
            60,
        )
        .unwrap();
        sandbox_sessions::register_pids(&conn, &session.id, &[4242]).unwrap();
        let vk = [0u8; 32];
        let prefix = minimal_client_hello_with_sni("api.example.com");
        match evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(4242), &prefix) {
            TransparentGateResult::Ok(ok) => assert_eq!(ok.session_id, session.id),
            TransparentGateResult::Deny { .. } => panic!("expected allow"),
        }
    }

    #[test]
    fn gate_denies_unregistered_pid() {
        let conn = mem_conn();
        let bucket_id = seed_bucket(&conn, r#"["api.example.com"]"#);
        let vk = [0u8; 32];
        let prefix = minimal_client_hello_with_sni("api.example.com");
        assert!(matches!(
            evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(9999), &prefix),
            TransparentGateResult::Deny { .. }
        ));
    }

    #[test]
    fn gate_denies_host_not_in_allowlist() {
        let conn = mem_conn();
        let bucket_id = seed_bucket(&conn, r#"["allowed.com"]"#);
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            "grant-1",
            "fp",
            None,
            60,
        )
        .unwrap();
        sandbox_sessions::register_pids(&conn, &session.id, &[4242]).unwrap();
        let vk = [0u8; 32];
        let prefix = minimal_client_hello_with_sni("blocked.com");
        assert!(matches!(
            evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(4242), &prefix),
            TransparentGateResult::Deny {
                reason: "host_denied",
                ..
            }
        ));
    }

    #[test]
    fn gate_denies_revoked_session() {
        let conn = mem_conn();
        let bucket_id = seed_bucket(&conn, r#"["api.example.com"]"#);
        let session = sandbox_sessions::create_session(
            &conn,
            &bucket_id,
            "grant-1",
            "fp",
            None,
            60,
        )
        .unwrap();
        sandbox_sessions::register_pids(&conn, &session.id, &[4242]).unwrap();
        sandbox_sessions::revoke_session(&conn, &session.id).unwrap();
        let vk = [0u8; 32];
        let prefix = minimal_client_hello_with_sni("api.example.com");
        assert!(matches!(
            evaluate_transparent_gate(&conn, &vk, &bucket_id, Some(4242), &prefix),
            TransparentGateResult::Deny { .. }
        ));
    }
}