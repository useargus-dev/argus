//! List active sandbox sessions via IPC.

use anyhow::Result;
use ipc_client::{sandbox_list, DEFAULT_TIMEOUT};

pub async fn run() -> Result<()> {
    let sessions = sandbox_list(DEFAULT_TIMEOUT)?;
    if sessions.is_empty() {
        println!("No active sandbox sessions.");
        return Ok(());
    }
    println!(
        "{:<40} {:<38} {:>6}  {}",
        "SESSION", "BUCKET", "PIDS", "EXPIRES"
    );
    println!("{}", "-".repeat(110));
    for s in sessions {
        let preview = s
            .command_preview
            .as_deref()
            .unwrap_or("-");
        let pid_list = if s.pids.is_empty() {
            "-".to_string()
        } else {
            s.pids
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "{:<40} {:<38} {:>6}  {}  {}",
            s.session_id,
            truncate(&s.bucket_id, 36),
            pid_list,
            s.expires_at,
            truncate(preview, 40),
        );
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
