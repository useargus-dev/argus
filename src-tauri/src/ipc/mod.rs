mod handler;
pub mod peer;
mod protocol;
mod server;

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::sessions::PendingApprovalStore;

pub use peer::VerifiedClient;
pub use server::IpcServerHandle;

pub struct IpcRuntime {
    pending: Arc<PendingApprovalStore>,
    server: Mutex<Option<IpcServerHandle>>,
}

impl Default for IpcRuntime {
    fn default() -> Self {
        Self {
            pending: Arc::new(PendingApprovalStore::default()),
            server: Mutex::new(None),
        }
    }
}

impl IpcRuntime {
    pub fn pending(&self) -> Arc<PendingApprovalStore> {
        self.pending.clone()
    }

    pub fn start(&self, app: &AppHandle) -> Result<(), String> {
        let mut guard = self.server.lock().map_err(|_| "ipc lock poisoned")?;
        if guard.is_some() {
            return Ok(());
        }
        let handle = server::start(app.clone(), self.pending.clone())?;
        *guard = Some(handle);
        Ok(())
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.server.lock() {
            if let Some(h) = guard.take() {
                h.stop();
            }
        }
    }
}

pub fn start_for_app(app: &AppHandle) {
    let ipc = app.state::<IpcRuntime>();
    if let Err(e) = ipc.start(app) {
        eprintln!("failed to start argus ipc: {e}");
    }
}

pub fn stop_for_app(app: &AppHandle) {
    let ipc = app.state::<IpcRuntime>();
    ipc.stop();
}
