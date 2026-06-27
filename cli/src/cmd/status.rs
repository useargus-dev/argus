use ipc_client::{ipc_endpoint, ping, DEFAULT_TIMEOUT, IpcClientError};
use clap::Args;

#[derive(Args, Default)]
pub struct StatusArgs {
    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: StatusArgs) -> anyhow::Result<()> {
    let connected = ping(DEFAULT_TIMEOUT).unwrap_or(false);
    let endpoint = ipc_endpoint();

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "connected": connected,
                "endpoint": endpoint,
            })
        );
        return Ok(());
    }

    if connected {
        println!("✓ Argus IPC reachable at {endpoint}");
        println!("  Ready for `argus run` (approve CLI access if prompted).");
    } else {
        println!("✗ Argus not reachable at {endpoint}");
        println!("  Start Argus, sign in, then retry.");
        return Err(IpcClientError::SocketNotFound {
            path: endpoint,
            hint: "Start Argus and sign in.".into(),
        }
        .into());
    }
    Ok(())
}
