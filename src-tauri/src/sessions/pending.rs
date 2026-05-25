use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAccessRequestEvent {
    pub request_id: String,
    pub bucket_id: String,
    pub bucket_name: String,
    pub fingerprint: String,
    pub pid: u32,
    pub exe_path: String,
    pub cwd: String,
    pub cwd_verified: bool,
    pub run_args: String,
    pub git_remote: Option<String>,
    pub process_name: String,
    pub machine_id: String,
    pub access_ttl_minutes: i64,
    pub created_at: String,
}

#[derive(Debug)]
pub enum PendingDecision {
    Accept { ttl_minutes: i64 },
    Deny,
}

struct PendingEntry {
    pub event: ClientAccessRequestEvent,
    pub responder: oneshot::Sender<PendingDecision>,
    pub created: Instant,
}

#[derive(Default)]
pub struct PendingApprovalStore {
    inner: Mutex<HashMap<String, PendingEntry>>,
}

impl PendingApprovalStore {
    pub fn register(
        &self,
        event: ClientAccessRequestEvent,
    ) -> (oneshot::Receiver<PendingDecision>, ClientAccessRequestEvent) {
        let (tx, rx) = oneshot::channel();
        let entry = PendingEntry {
            event: event.clone(),
            responder: tx,
            created: Instant::now(),
        };
        self.inner
            .lock()
            .expect("pending store poisoned")
            .insert(event.request_id.clone(), entry);
        (rx, event)
    }

    pub fn respond(&self, request_id: &str, decision: PendingDecision) -> bool {
        let entry = self
            .inner
            .lock()
            .expect("pending store poisoned")
            .remove(request_id);
        if let Some(entry) = entry {
            let _ = entry.responder.send(decision);
            return true;
        }
        false
    }

    pub fn list(&self) -> Vec<ClientAccessRequestEvent> {
        self.prune_expired();
        self.inner
            .lock()
            .expect("pending store poisoned")
            .values()
            .map(|e| e.event.clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.prune_expired();
        self.inner.lock().expect("pending store poisoned").len()
    }

    fn prune_expired(&self) {
        let mut guard = self.inner.lock().expect("pending store poisoned");
        guard.retain(|_, e| e.created.elapsed() < Duration::from_secs(900));
    }
}
