use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Context, Result};
use mitmproxy::intercept_conf::InterceptConf;
use mitmproxy::packet_sources::{PacketSourceConf, PacketSourceTask};
use mitmproxy::shutdown::{self, shutdown_task};
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinSet;

use crate::relay::RelayTask;

pub struct RedirectorHandle {
    shutdown_tx: tokio::sync::watch::Sender<()>,
    conf_tx: UnboundedSender<InterceptConf>,
    join: tokio::task::JoinHandle<()>,
}

impl RedirectorHandle {
    pub fn set_intercept(&self, spec: &str) -> Result<()> {
        let conf = InterceptConf::try_from(spec).context("invalid intercept spec")?;
        self.conf_tx
            .send(conf)
            .map_err(|_| anyhow::anyhow!("redirector event loop stopped"))
    }

    pub async fn stop(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.join.await;
    }
}

async fn start_redirector<T>(conf: T, target_port: u16, relay_secret: Option<[u8; 32]>) -> Result<RedirectorHandle>
where
    T: PacketSourceConf<Data = UnboundedSender<InterceptConf>>,
    T::Task: PacketSourceTask,
{
    crate::relay::init_relay_secret(relay_secret);
    let target = SocketAddr::from((Ipv4Addr::LOCALHOST, target_port));

    let (transport_events_tx, transport_events_rx) = mpsc::channel(256);
    let (transport_commands_tx, transport_commands_rx) = mpsc::unbounded_channel();
    let (shutdown_start_tx, shutdown_start_rx) = shutdown::channel();
    let shutdown_relay = shutdown_start_rx.clone();

    let (packet_source_task, conf_tx) = conf
        .build(
            transport_events_tx,
            transport_commands_rx,
            shutdown_start_rx,
        )
        .await?;

    let relay = RelayTask::new(
        transport_events_rx,
        transport_commands_tx,
        target,
        shutdown_relay,
    );

    let mut tasks = JoinSet::new();
    tasks.spawn(async move { packet_source_task.run().await });
    tasks.spawn(async move { relay.run().await });

    let (shutdown_done_tx, mut shutdown_done_rx) = shutdown::channel();
    tokio::spawn(shutdown_task(tasks, shutdown_done_tx));

    let join = tokio::spawn(async move {
        shutdown_done_rx.recv().await;
    });

    Ok(RedirectorHandle {
        shutdown_tx: shutdown_start_tx,
        conf_tx,
        join,
    })
}

#[cfg(target_os = "linux")]
pub async fn start_linux_redirector(
    executable_path: PathBuf,
    target_port: u16,
    relay_secret: Option<[u8; 32]>,
) -> Result<RedirectorHandle> {
    use mitmproxy::packet_sources::linux::LinuxConf;
    if !executable_path.exists() {
        anyhow::bail!(
            "Linux redirector not found at {}",
            executable_path.display()
        );
    }
    start_redirector(LinuxConf { executable_path }, target_port, relay_secret).await
}

#[cfg(not(target_os = "linux"))]
pub async fn start_linux_redirector(
    _executable_path: PathBuf,
    _target_port: u16,
    _relay_secret: Option<[u8; 32]>,
) -> Result<RedirectorHandle> {
    anyhow::bail!("Linux redirector is only available on Linux")
}

#[cfg(windows)]
pub async fn start_windows_redirector(
    executable_path: PathBuf,
    target_port: u16,
    relay_secret: Option<[u8; 32]>,
) -> Result<RedirectorHandle> {
    use mitmproxy::packet_sources::windows::WindowsConf;
    if !executable_path.exists() {
        anyhow::bail!(
            "Windows redirector not found at {}",
            executable_path.display()
        );
    }
    start_redirector(WindowsConf { executable_path }, target_port, relay_secret).await
}

#[cfg(not(windows))]
pub async fn start_windows_redirector(
    _executable_path: PathBuf,
    _target_port: u16,
    _relay_secret: Option<[u8; 32]>,
) -> Result<RedirectorHandle> {
    anyhow::bail!("Windows redirector is only available on Windows")
}

/// True when the current process already has OS privileges for capture.
pub fn has_capture_privileges() -> bool {
    #[cfg(target_os = "linux")]
    {
        nix::unistd::geteuid().is_root()
    }
    #[cfg(windows)]
    {
        is_windows_elevated()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        false
    }
}

/// User-facing notice when elevation will be requested at redirector start (not a hard failure).
pub fn elevation_notice() -> Option<String> {
    if has_capture_privileges() {
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        Some(
            "You may be prompted for sudo to start network capture (once per sudo timeout)."
                .into(),
        )
    }
    #[cfg(windows)]
    {
        Some(
            "A Windows Administrator (UAC) prompt may appear to start network capture.".into(),
        )
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        Some(format!(
            "argus run network capture is not supported on {}",
            std::env::consts::OS
        ))
    }
}

/// Deprecated alias — use [`elevation_notice`] for UX hints.
pub fn unavailable_reason() -> Option<String> {
    elevation_notice()
}

#[cfg(windows)]
fn is_windows_elevated() -> bool {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Security::GetTokenInformation;
    use windows_sys::Win32::Security::TokenElevation;
    use windows_sys::Win32::Security::TOKEN_ELEVATION;
    use windows_sys::Win32::Security::TOKEN_QUERY;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    use windows_sys::Win32::System::Threading::OpenProcessToken;

    unsafe {
        let mut token: HANDLE = std::mem::zeroed();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        let _ = windows_sys::Win32::Foundation::CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}
