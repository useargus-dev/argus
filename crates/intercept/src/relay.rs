use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use anyhow::Result;
use mitmproxy::messages::{ConnectionId, TransportCommand, TransportEvent, TunnelInfo};
use mitmproxy::shutdown;
use protocol::capture_log;
use protocol::relay_frame;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

static RELAY_NONCE: AtomicU64 = AtomicU64::new(1);
static RELAY_SECRET: OnceLock<Option<[u8; 32]>> = OnceLock::new();

/// Install per-session relay HMAC secret in-process (never via environment).
pub fn init_relay_secret(secret: Option<[u8; 32]>) {
    let _ = RELAY_SECRET.set(secret);
}

fn relay_secret() -> Option<[u8; 32]> {
    RELAY_SECRET.get().and_then(|s| s.clone())
}

pub struct RelayTask {
    transport_events: mpsc::Receiver<TransportEvent>,
    /// Shared command channel for smoltcp (ConnectionEstablished uses `command_tx: None`).
    transport_commands: mpsc::UnboundedSender<TransportCommand>,
    target: SocketAddr,
    shutdown: shutdown::Receiver,
}

impl RelayTask {
    pub fn new(
        transport_events: mpsc::Receiver<TransportEvent>,
        transport_commands: mpsc::UnboundedSender<TransportCommand>,
        target: SocketAddr,
        shutdown: shutdown::Receiver,
    ) -> Self {
        Self {
            transport_events,
            transport_commands,
            target,
            shutdown,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        capture_log::log("relay", format!("started → {}", self.target));
        let active = Arc::new(Mutex::new(HashMap::<ConnectionId, tokio::task::JoinHandle<()>>::new()));

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => break,
                event = self.transport_events.recv() => {
                    let Some(event) = event else { break };
                    let TransportEvent::ConnectionEstablished {
                        connection_id,
                        src_addr,
                        dst_addr,
                        command_tx,
                        tunnel_info,
                        ..
                    } = event;
                    if !connection_id.is_tcp() {
                        continue;
                    }
                    let relay_pid = match &tunnel_info {
                        TunnelInfo::LocalRedirector {
                            pid: Some(pid), ..
                        } => Some(*pid),
                        other => {
                            capture_log::log(
                                "relay",
                                format!(
                                    "conn {connection_id} {src_addr} → {dst_addr}: no captured pid ({other:?})"
                                ),
                            );
                            None
                        }
                    };
                    if let Some(pid) = relay_pid {
                        capture_log::log(
                            "relay",
                            format!("conn {connection_id} {src_addr} → {dst_addr}: captured pid={pid}"),
                        );
                    }
                    let cmd_tx = command_tx
                        .unwrap_or_else(|| self.transport_commands.clone());
                    let target = self.target;
                    let active_remove = active.clone();
                    let handle = tokio::spawn(async move {
                        if let Err(e) =
                            relay_connection(connection_id, cmd_tx, target, relay_pid).await
                        {
                            capture_log::log(
                                "relay",
                                format!("conn {connection_id} ended: {e:#}"),
                            );
                        }
                        active_remove.lock().await.remove(&connection_id);
                    });
                    active.lock().await.insert(connection_id, handle);
                }
            }
        }

        capture_log::log("relay", "shutting down");
        let mut guard = active.lock().await;
        for (_, handle) in guard.drain() {
            handle.abort();
        }
        Ok(())
    }
}

async fn relay_connection(
    connection_id: ConnectionId,
    command_tx: mpsc::UnboundedSender<TransportCommand>,
    target: SocketAddr,
    captured_pid: Option<u32>,
) -> Result<()> {
    let upstream = TcpStream::connect(target).await.map_err(|e| {
        capture_log::log(
            "relay",
            format!("conn {connection_id}: connect {target} failed: {e}"),
        );
        e
    })?;
    capture_log::log(
        "relay",
        format!("conn {connection_id}: connected to {target}"),
    );

    let (mut upstream_read, mut upstream_write) = upstream.into_split();

    let client_to_proxy = async {
        let mut relay_header_sent = false;
        let mut iteration = 0u32;

        loop {
            iteration += 1;
            let data = read_smoltcp(connection_id, &command_tx).await?;
            if data.is_empty() {
                capture_log::log(
                    "relay",
                    format!("conn {connection_id}: client EOF (iter {iteration})"),
                );
                let _ = upstream_write.shutdown().await;
                break;
            }

            if !relay_header_sent {
                if let Some(pid) = captured_pid {
                    if let Some(secret) = relay_secret() {
                        let nonce = RELAY_NONCE.fetch_add(1, Ordering::Relaxed);
                        let hdr = relay_frame::encode_signed(&secret, pid, nonce);
                        capture_log::log(
                            "relay",
                            format!(
                                "conn {connection_id}: sending signed relay header pid={pid} ({} B TLS next)",
                                data.len()
                            ),
                        );
                        upstream_write.write_all(&hdr).await?;
                    } else {
                        capture_log::log(
                            "relay",
                            format!(
                                "conn {connection_id}: relay secret missing; forwarding {} B without relay header",
                                data.len()
                            ),
                        );
                    }
                } else {
                    capture_log::log(
                        "relay",
                        format!(
                            "conn {connection_id}: no relay header; forwarding {} B to proxy",
                            data.len()
                        ),
                    );
                }
                relay_header_sent = true;
            } else {
                capture_log::log(
                    "relay",
                    format!(
                        "conn {connection_id}: client→proxy {} B (iter {iteration})",
                        data.len()
                    ),
                );
            }

            upstream_write.write_all(&data).await?;
        }
        Ok::<(), anyhow::Error>(())
    };

    let proxy_to_client = async {
        let mut iteration = 0u32;

        loop {
            iteration += 1;
            let mut buf = vec![0u8; 16 * 1024];
            let n = upstream_read.read(&mut buf).await?;
            if n == 0 {
                capture_log::log(
                    "relay",
                    format!("conn {connection_id}: proxy EOF (iter {iteration})"),
                );
                let _ = command_tx.send(TransportCommand::CloseConnection(connection_id, true));
                break;
            }
            capture_log::log(
                "relay",
                format!("conn {connection_id}: proxy→client {n} B (iter {iteration})"),
            );
            if command_tx
                .send(TransportCommand::WriteData(
                    connection_id,
                    buf[..n].to_vec(),
                ))
                .is_err()
            {
                capture_log::log("relay", format!("conn {connection_id}: WriteData channel closed"));
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::try_join!(client_to_proxy, proxy_to_client)?;
    Ok(())
}

async fn read_smoltcp(
    connection_id: ConnectionId,
    command_tx: &mpsc::UnboundedSender<TransportCommand>,
) -> Result<Vec<u8>> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    command_tx
        .send(TransportCommand::ReadData(connection_id, 16 * 1024, reply_tx))
        .map_err(|_| anyhow::anyhow!("ReadData channel closed"))?;
    Ok(reply_rx.await.unwrap_or_default())
}
