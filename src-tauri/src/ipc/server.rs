use std::sync::Arc;
use std::thread::JoinHandle;

use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::watch;

use crate::infra::db;
use crate::error::AppError;
use crate::ipc::handler;
use crate::ipc::peer;
use crate::ipc::protocol::{IpcRequest, IpcResponse};
use crate::sessions::PendingApprovalStore;

pub struct IpcServerHandle {
    shutdown_tx: watch::Sender<bool>,
    join: Option<JoinHandle<()>>,
}

impl IpcServerHandle {
    pub fn stop(mut self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        remove_socket_file();
    }
}

pub fn start(
    app: AppHandle,
    pending_store: Arc<PendingApprovalStore>,
) -> Result<IpcServerHandle, String> {
    remove_socket_file();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let join = std::thread::Builder::new()
        .name("argus-ipc".into())
        .spawn(move || {
            if let Err(e) = run_server(app, pending_store, shutdown_rx) {
                eprintln!("argus ipc server exited: {e}");
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(IpcServerHandle {
        shutdown_tx,
        join: Some(join),
    })
}

fn run_server(
    app: AppHandle,
    pending_store: Arc<PendingApprovalStore>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .map_err(|e| e.to_string())?;

    rt.block_on(async move {
        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            #[cfg(unix)]
            {
                if let Err(e) = serve_unix(&app, &pending_store, &mut shutdown_rx).await {
                    if !*shutdown_rx.borrow() {
                        eprintln!("argus ipc accept error: {e}");
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }

            #[cfg(windows)]
            {
                if let Err(e) = serve_windows_pipe(&app, &pending_store, &mut shutdown_rx).await {
                    if !*shutdown_rx.borrow() {
                        eprintln!("argus ipc accept error: {e}");
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }

            #[cfg(not(any(unix, windows)))]
            {
                let _ = (&app, &pending_store, &mut shutdown_rx);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }
        remove_socket_file();
        Ok(())
    })
}

#[cfg(unix)]
async fn serve_unix(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    use tokio::net::UnixListener;

    let path = socket_path();
    db::ensure_argus_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
    crate::util::fs::harden_path(&path, false).map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }
            accept = listener.accept() => {
                let (stream, _) = accept.map_err(|e| e.to_string())?;
                let app = app.clone();
                let store = pending_store.clone();
                tokio::spawn(async move {
                    handle_unix_stream(app, store, stream).await;
                });
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
async fn serve_windows_pipe(
    app: &AppHandle,
    pending_store: &Arc<PendingApprovalStore>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ServerOptions;

    loop {
        if *shutdown_rx.borrow() {
            break;
        }

        let server = ServerOptions::new()
            .create(r"\\.\pipe\argus")
            .map_err(|e| e.to_string())?;

        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }
            ready = server.connect() => {
                ready.map_err(|e| e.to_string())?;
                let app = app.clone();
                let store = pending_store.clone();
                tokio::spawn(async move {
                    handle_windows_pipe(app, store, server).await;
                });
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
async fn handle_unix_stream(
    app: AppHandle,
    pending_store: Arc<PendingApprovalStore>,
    mut stream: tokio::net::UnixStream,
) {
    use std::os::unix::io::AsRawFd;

    let fd = stream.as_raw_fd();

    let mut buf = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        if reader.read_line(&mut buf).await.is_err() || buf.is_empty() {
            return;
        }
    }

    let fallback_cwd = parse_fallback_cwd(&buf);
    let peer = match peer::from_connected_stream(&stream, fallback_cwd.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            let response = peer_error_response(&e);
            let _ = stream.write_all(format!("{response}\n").as_bytes()).await;
            return;
        }
    };
    let _ = fd;
    let response = handler::handle_request(&app, &pending_store, &peer, &buf).await;
    let _ = stream.write_all(format!("{response}\n").as_bytes()).await;
}

#[cfg(windows)]
async fn handle_windows_pipe(
    app: AppHandle,
    pending_store: Arc<PendingApprovalStore>,
    mut server: tokio::net::windows::named_pipe::NamedPipeServer,
) {
    let mut buf = String::new();
    {
        let mut reader = BufReader::new(&mut server);
        if reader.read_line(&mut buf).await.is_err() || buf.is_empty() {
            return;
        }
    }

    let fallback_cwd = parse_fallback_cwd(&buf);
    let peer = match peer::from_connected_stream(&server, fallback_cwd.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            let response = peer_error_response(&e);
            let _ = server.write_all(format!("{response}\n").as_bytes()).await;
            return;
        }
    };
    let response = handler::handle_request(&app, &pending_store, &peer, &buf).await;
    let _ = server.write_all(format!("{response}\n").as_bytes()).await;
}

fn parse_fallback_cwd(line: &str) -> Option<String> {
    let req: IpcRequest = serde_json::from_str(line.trim()).ok()?;
    req.cwd
}

fn peer_error_response(err: &AppError) -> String {
    IpcResponse::Error {
        request_id: String::new(),
        code: err.code().to_string(),
        message: err.to_string(),
    }
    .to_line()
}

fn socket_path() -> std::path::PathBuf {
    db::argus_dir().join("argus.sock")
}

fn remove_socket_file() {
    #[cfg(unix)]
    {
        let path = socket_path();
        let _ = std::fs::remove_file(path);
    }
}
