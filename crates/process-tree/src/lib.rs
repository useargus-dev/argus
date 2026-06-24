//! Walk process trees and watch for new child PIDs (uvicorn `--reload`).

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessTreeError {
    #[error("process tree walk failed: {0}")]
    Walk(String),
}

/// Collect all descendant PIDs of `root` (including root).
pub fn descendants(root: u32) -> Result<Vec<u32>, ProcessTreeError> {
    let mut all = HashSet::new();
    all.insert(root);
    let mut queue = VecDeque::from([root]);
    while let Some(pid) = queue.pop_front() {
        for child in direct_children(pid)? {
            if all.insert(child) {
                queue.push_back(child);
            }
        }
    }
    Ok(all.into_iter().collect())
}

pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl WatcherHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Spawn a background watcher; calls `on_new` when new PIDs appear under `root`.
pub fn spawn_watcher<F>(root: u32, mut on_new: F) -> WatcherHandle
where
    F: FnMut(Vec<u32>) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let join = thread::spawn(move || {
        let mut known = descendants(root).unwrap_or_else(|_| vec![root]);
        let mut known_set: HashSet<u32> = known.iter().copied().collect();
        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(500));
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            let current = descendants(root).unwrap_or_default();
            let fresh: Vec<u32> = current
                .into_iter()
                .filter(|p| known_set.insert(*p))
                .collect();
            if !fresh.is_empty() {
                known.extend(&fresh);
                on_new(fresh);
            }
        }
    });
    WatcherHandle {
        stop,
        join: Some(join),
    }
}

#[cfg(unix)]
fn direct_children(pid: u32) -> Result<Vec<u32>, ProcessTreeError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir("/proc").map_err(|e| ProcessTreeError::Walk(e.to_string()))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Ok(child_pid) = name.to_string_lossy().parse::<u32>() else {
            continue;
        };
        let stat_path = entry.path().join("stat");
        let Ok(stat) = std::fs::read_to_string(&stat_path) else {
            continue;
        };
        // Format: pid (comm) state ppid ...
        let rparen = stat.rfind(')').ok_or_else(|| ProcessTreeError::Walk("bad stat".into()))?;
        let rest = stat[rparen + 2..].split_whitespace().collect::<Vec<_>>();
        if rest.len() < 2 {
            continue;
        }
        if rest[1].parse::<u32>().ok() == Some(pid) {
            out.push(child_pid);
        }
    }
    Ok(out)
}

#[cfg(windows)]
fn direct_children(pid: u32) -> Result<Vec<u32>, ProcessTreeError> {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Err(ProcessTreeError::Walk(
                "CreateToolhelp32Snapshot failed".into(),
            ));
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut out = Vec::new();
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if entry.th32ParentProcessID == pid {
                    out.push(entry.th32ProcessID);
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        let _ = windows_sys::Win32::Foundation::CloseHandle(snap);
        Ok(out)
    }
}

#[cfg(not(any(unix, windows)))]
fn direct_children(_pid: u32) -> Result<Vec<u32>, ProcessTreeError> {
    Ok(vec![])
}
