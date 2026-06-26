use anyhow::{Context, Result, bail};
use log::{Level, debug, error, log};
use std::io::IsTerminal;
use std::io::Error;
use std::net::Shutdown;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::str::FromStr;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::intercept_conf::InterceptConf;
use crate::messages::{TransportCommand, TransportEvent};
use crate::packet_sources::{PacketSourceConf, PacketSourceTask, forward_packets};
use crate::shutdown;
use tempfile::{TempDir, tempdir};
use tokio::net::UnixDatagram;
use tokio::process::Command;
use tokio::time::timeout;

async fn start_redirector(
    executable: &Path,
    listener_addr: &Path,
    shutdown: shutdown::Receiver,
) -> Result<PathBuf> {
    if is_root() {
        debug!("Already root; starting redirector without sudo...");
        return spawn_redirector(executable, listener_addr, shutdown, None).await;
    }

    if which_sudo().is_none() {
        bail!(
            "Network capture requires sudo, but sudo was not found. \
             Install sudo or use --no-proxy to inject secrets without OS capture."
        );
    }

    debug!("Elevating privileges via sudo...");
    ensure_sudo_credentials().await?;

    debug!("Starting mitmproxy-linux-redirector...");
    spawn_redirector(executable, listener_addr, shutdown, Some("sudo")).await
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn which_sudo() -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg("sudo")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            }
        })
}

async fn ensure_sudo_credentials() -> Result<()> {
    let mut sudo = Command::new("sudo")
        .arg("-v")
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to run sudo.")?;
    sudo.stdin.take();
    let status = sudo.wait().await.context("Failed to wait for sudo.")?;
    if !status.success() {
        bail!(
            "Network capture requires sudo. Approve the sudo prompt, configure polkit, \
             or use --no-proxy to inject secrets without OS capture."
        );
    }
    Ok(())
}

async fn spawn_redirector(
    executable: &Path,
    listener_addr: &Path,
    shutdown: shutdown::Receiver,
    via_sudo: Option<&str>,
) -> Result<PathBuf> {
    let mut redirector_process = if let Some(sudo) = via_sudo {
        let mut cmd = Command::new(sudo);
        cmd.arg("--preserve-env");
        if !std::io::stdin().is_terminal() {
            cmd.arg("--non-interactive");
        }
        cmd.arg(executable).arg(listener_addr);
        cmd
    } else {
        let mut cmd = Command::new(executable);
        cmd.arg(listener_addr);
        cmd
    }
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .with_context(|| {
        format!(
            "Failed to launch redirector at {}",
            executable.display()
        )
    })?;

    let stdout = redirector_process.stdout.take().unwrap();
    let stderr = redirector_process.stderr.take().unwrap();
    let shutdown2 = shutdown.clone();
    tokio::spawn(async move {
        let mut stderr = BufReader::new(stderr).lines();
        let mut level = Level::Error;
        while let Ok(Some(line)) = stderr.next_line().await {
            if shutdown2.is_shutting_down() {
                // We don't want to log during exit, https://github.com/vorner/pyo3-log/issues/30
                eprintln!("{line}");
                continue;
            }

            let new_level = line
                .strip_prefix("[")
                .and_then(|s| s.split_once(" "))
                .and_then(|(level, line)| {
                    Level::from_str(level)
                        .ok()
                        .map(|l| (l, line.trim_ascii_start()))
                });
            if let Some((l, line)) = new_level {
                level = l;
                log!(level, "[{line}");
            } else {
                log!(level, "{line}");
            }
        }
    });
    tokio::spawn(async move {
        match redirector_process.wait().await {
            Ok(status) if status.success() => {
                if shutdown.is_shutting_down() {
                    // We don't want to log during exit, https://github.com/vorner/pyo3-log/issues/30
                } else {
                    debug!("[linux-redirector] exited successfully.")
                }
            }
            other => {
                if shutdown.is_shutting_down() {
                    eprintln!("[linux-redirector] exited during shutdown: {other:?}")
                } else {
                    error!("[linux-redirector] exited: {other:?}")
                }
            }
        }
    });

    timeout(
        Duration::from_secs(5),
        BufReader::new(stdout).lines().next_line(),
    )
    .await
    .context("failed to establish connection to Linux redirector")?
    .context("failed to read redirector stdout")?
    .map(PathBuf::from)
    .context("redirector did not produce stdout")
}

pub struct LinuxConf {
    pub executable_path: PathBuf,
}

// We implement AsyncRead/AsyncWrite for UnixDatagram to have a common interface
// with Windows' NamedPipeServer.
pub struct AsyncUnixDatagram(UnixDatagram);

impl AsyncRead for AsyncUnixDatagram {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.0.poll_recv(cx, buf)
    }
}
impl AsyncWrite for AsyncUnixDatagram {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, Error>> {
        self.0.poll_send(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        self.0.poll_send_ready(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        Poll::Ready(self.0.shutdown(Shutdown::Write))
    }
}

impl PacketSourceConf for LinuxConf {
    type Task = LinuxTask;
    type Data = UnboundedSender<InterceptConf>;

    fn name(&self) -> &'static str {
        "Linux proxy"
    }

    async fn build(
        self,
        transport_events_tx: Sender<TransportEvent>,
        transport_commands_rx: UnboundedReceiver<TransportCommand>,
        shutdown: shutdown::Receiver,
    ) -> Result<(Self::Task, Self::Data)> {
        let datagram_dir = tempdir().context("failed to create temp dir")?;

        let channel = UnixDatagram::bind(datagram_dir.path().join("mitmproxy"))?;
        let dst =
            start_redirector(&self.executable_path, datagram_dir.path(), shutdown.clone()).await?;

        channel
            .connect(&dst)
            .with_context(|| format!("Failed to connect to redirector at {}", dst.display()))?;

        let (conf_tx, conf_rx) = unbounded_channel();

        Ok((
            LinuxTask {
                datagram_dir,
                channel: AsyncUnixDatagram(channel),
                transport_events_tx,
                transport_commands_rx,
                conf_rx,
                shutdown,
            },
            conf_tx,
        ))
    }
}

pub struct LinuxTask {
    datagram_dir: TempDir,
    channel: AsyncUnixDatagram,
    transport_events_tx: Sender<TransportEvent>,
    transport_commands_rx: UnboundedReceiver<TransportCommand>,
    conf_rx: UnboundedReceiver<InterceptConf>,
    shutdown: shutdown::Receiver,
}

impl PacketSourceTask for LinuxTask {
    async fn run(self) -> Result<()> {
        forward_packets(
            self.channel,
            self.transport_events_tx,
            self.transport_commands_rx,
            self.conf_rx,
            self.shutdown,
        )
        .await?;
        drop(self.datagram_dir);
        Ok(())
    }
}
