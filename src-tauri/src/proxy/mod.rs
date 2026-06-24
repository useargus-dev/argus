pub mod auth;
pub mod ca;
pub mod peer_tcp;
pub mod rewrite;
pub mod server;
pub mod tls_sni;
pub mod transparent;

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::infra::db::buckets;
use crate::proxy::ca::ensure_ca_material;
use crate::proxy::server::{start_bucket_proxy, ProxyServerHandle};
use crate::state::AppState;
use crate::util::session as app_session;

pub struct ProxyRuntime {
    servers: Mutex<HashMap<String, ProxyServerHandle>>,
}

impl Default for ProxyRuntime {
    fn default() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
        }
    }
}

impl ProxyRuntime {
    pub fn start_bucket(&self, app: &AppHandle, bucket_id: &str, port: u16) -> Result<(), String> {
        let _ = ensure_ca_material().map_err(|e| e.to_string())?;
        let handle = start_bucket_proxy(app.clone(), bucket_id.to_string(), port)
            .map_err(|e| e.to_string())?;
        let mut guard = self.servers.lock().map_err(|_| "proxy lock poisoned")?;
        if let Some(old) = guard.remove(bucket_id) {
            old.stop();
        }
        guard.insert(bucket_id.to_string(), handle);
        Ok(())
    }

    pub fn stop_bucket(&self, bucket_id: &str) {
        if let Ok(mut guard) = self.servers.lock() {
            if let Some(h) = guard.remove(bucket_id) {
                h.stop();
            }
        }
    }

    pub fn stop_all(&self) {
        if let Ok(mut guard) = self.servers.lock() {
            for (_, h) in guard.drain() {
                h.stop();
            }
        }
    }

    /// Start the bucket proxy listener if not already running.
    pub fn ensure_bucket_running(
        app: &AppHandle,
        bucket_id: &str,
        port: u16,
    ) -> Result<(), String> {
        let proxy = app.state::<ProxyRuntime>();
        let already = proxy
            .servers
            .lock()
            .map_err(|_| "proxy lock poisoned".to_string())?
            .contains_key(bucket_id);
        if already {
            return Ok(());
        }
        proxy.start_bucket(app, bucket_id, port)
    }

    pub fn sync_enabled_buckets(app: &AppHandle) -> Result<(), String> {
        let _ = ensure_ca_material().map_err(|e| e.to_string())?;
        let state = app.state::<AppState>();
        let list: Vec<(String, u16)> = app_session::with_db(&state, |conn, _inner| {
            buckets::list_proxy_enabled_buckets(conn)
        })
        .map_err(|e| e.to_string())?;

        let proxy = app.state::<ProxyRuntime>();
        proxy.stop_all();

        for (bucket_id, port) in list {
            proxy.start_bucket(app, &bucket_id, port)?;
        }
        Ok(())
    }
}

pub fn start_for_app(app: &AppHandle) {
    if let Err(e) = ProxyRuntime::sync_enabled_buckets(app) {
        eprintln!("failed to start argus proxies: {e}");
    }
}

pub fn stop_for_app(app: &AppHandle) {
    let proxy = app.state::<ProxyRuntime>();
    proxy.stop_all();
}
