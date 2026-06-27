//! Argus CLI sidecar — `argus run`, `status`, `sessions`.

mod cmd;
mod config;
mod paths;
mod spawn;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "argus", about = "Argus secrets vault CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command with OS-level secret injection
    Run(cmd::run::RunArgs),
    /// Show Argus connectivity and proxy status
    Status(cmd::status::StatusArgs),
    /// List active sandbox sessions (M5+)
    Sessions,
}

#[tokio::main]
async fn main() {
    init_logging();
    if let Err(e) = run().await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn init_logging() {
    if std::env::var("RUST_LOG").is_err() {
        // relay + mitmproxy stack when capture trace is on
        unsafe { std::env::set_var("RUST_LOG", "info,intercept=debug,mitmproxy=info") };
    }
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init();
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => cmd::status::run(cmd::status::StatusArgs::default()).await,
        Some(Commands::Run(args)) => cmd::run::run(args).await,
        Some(Commands::Status(args)) => cmd::status::run(args).await,
        Some(Commands::Sessions) => cmd::sessions::run().await,
    }
}
