use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use ipc_client::{
    sandbox_create, sandbox_register_pids, sandbox_revoke, DEFAULT_TIMEOUT, IpcClientError,
};
use process_tree::spawn_watcher;
use redirector_core::{intercept_spec_for_pid, start_redirector};
use clap::Args;
use tokio::process::Command;

use crate::config::resolve_bucket;
use crate::paths::{argus_home, redirector_path};
use protocol::capture_log;

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Bucket UUID or display name
    #[arg(long)]
    pub bucket: Option<String>,
    /// Path to .env file
    #[arg(long, default_value = ".env")]
    pub env: PathBuf,
    /// Print intercept plan without executing
    #[arg(long)]
    pub dry_run: bool,
    /// Inject real secrets without OS capture
    #[arg(long)]
    pub no_proxy: bool,
    /// Command and arguments (use `--` when flags precede the command)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

pub async fn run(args: RunArgs) -> Result<()> {
    if args.command.is_empty() {
        anyhow::bail!("usage: argus run [FLAGS] [--] COMMAND [ARGS...]");
    }

    let bucket = resolve_bucket(args.bucket.as_deref(), &args.env)?;
    let command_preview = Some(args.command.join(" "));
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string());

    if args.dry_run {
        print_dry_run(&bucket.bucket_id, &args)?;
        return Ok(());
    }

    let session = sandbox_create(
        &bucket.bucket_id,
        &bucket.client_token,
        cwd.as_deref(),
        command_preview.as_deref(),
        DEFAULT_TIMEOUT,
    )
    .map_err(map_ipc_error)?;

    println!(
        "✓ Argus connected (bucket: {}, proxy: 127.0.0.1:{})",
        bucket.bucket_id, session.proxy_port
    );
    println!(
        "✓ Sandbox session: {} (expires {})",
        session.session_id, session.expires_at
    );

    if args.no_proxy {
        let code = run_child(&session.env, &session.ca_bundle_path, &session.session_id, &args.command).await?;
        std::process::exit(code);
    }

    if let Some(notice) = redirector_core::privilege_hint() {
        eprintln!("⚠ {notice}");
    }

    if capture_log::enabled() {
        eprintln!(
            "ℹ Capture trace on (stderr). Set ARGUS_CAPTURE_LOG=0 to silence. \
             Proxy/redirector logs: Argus desktop terminal + redirector console (debug build)."
        );
    }

    let redirector_path = redirector_path();
    capture_log::log(
        "run",
        format!(
            "starting redirector → 127.0.0.1:{} ({})",
            session.proxy_port,
            redirector_path.display()
        ),
    );
    let redirector = start_redirector(session.proxy_port, Some(&redirector_path))
        .await
        .context("failed to start network redirector")?;
    capture_log::log("run", "redirector IPC connected");

    let mut child_env = session.env.clone();
    apply_sandbox_env(&mut child_env, &session.ca_bundle_path, &session.session_id);
    capture_log::log(
        "run",
        format!(
            "sandbox env keys: {}",
            child_env.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
    );
    if !session.ca_bundle_path.is_empty() {
        capture_log::log("run", format!("CA bundle: {}", session.ca_bundle_path));
    }

    let mut cmd = build_command(&args.command, &child_env)?;
    // Spawn, then register intercept + sandbox PID before the child runs network I/O.
    let mut child = cmd.spawn().context("failed to spawn command")?;
    let root_pid = child.id().unwrap_or(0);
    capture_log::log("run", format!("spawned child pid={root_pid}"));

    redirector
        .set_intercept(&intercept_spec_for_pid(root_pid))
        .context("failed to set intercept spec")?;
    capture_log::log("run", format!("redirector intercept spec: Include PID {root_pid}"));

    sandbox_register_pids(&session.session_id, &[root_pid], DEFAULT_TIMEOUT)
        .map_err(map_ipc_error)?;
    capture_log::log(
        "run",
        format!("registered sandbox pids for session {}", session.session_id),
    );

    let session_id = session.session_id.clone();
    let _watcher = spawn_watcher(root_pid, move |fresh| {
        let _ = sandbox_register_pids(&session_id, &fresh, DEFAULT_TIMEOUT);
    });

    let exit_code = wait_with_signals(&mut child).await?;
    redirector.stop().await;
    let _ = sandbox_revoke(&session.session_id, DEFAULT_TIMEOUT);
    std::process::exit(exit_code);
}

fn print_dry_run(bucket_id: &str, args: &RunArgs) -> Result<()> {
    println!("Dry run — argus run plan:");
    println!("  Bucket:     {bucket_id}");
    println!("  Env file:   {}", args.env.display());
    println!("  ARGUS_HOME: {}", argus_home().display());
    println!("  Redirector: {}", redirector_path().display());
    if let Some(notice) = redirector_core::privilege_hint() {
        println!("  Privilege:  {notice}");
    } else {
        println!("  Privilege:  OK (already elevated)");
    }
    println!("  Command:    {}", args.command.join(" "));
    Ok(())
}

fn apply_sandbox_env(env: &mut HashMap<String, String>, ca: &str, session: &str) {
    env.insert("ARGUS_SANDBOX".into(), "1".into());
    env.insert("ARGUS_SANDBOX_SESSION".into(), session.to_string());
    if !ca.is_empty() {
        env.insert("SSL_CERT_FILE".into(), ca.to_string());
        env.insert("REQUESTS_CA_BUNDLE".into(), ca.to_string());
        env.insert("NODE_EXTRA_CA_CERTS".into(), ca.to_string());
        env.insert("CURL_CA_BUNDLE".into(), ca.to_string());
    }
}

fn build_command(argv: &[String], env: &HashMap<String, String>) -> Result<Command> {
    let program = argv.first().context("empty command")?;
    let mut cmd = Command::new(program);
    cmd.args(&argv[1..]);
    cmd.envs(env);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    Ok(cmd)
}

async fn run_child(
    env: &HashMap<String, String>,
    ca: &str,
    session_id: &str,
    command: &[String],
) -> Result<i32> {
    let mut child_env = env.clone();
    apply_sandbox_env(&mut child_env, ca, session_id);
    let mut cmd = build_command(command, &child_env)?;
    let mut child = cmd.spawn()?;
    wait_with_signals(&mut child).await
}

async fn wait_with_signals(child: &mut tokio::process::Child) -> Result<i32> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        use tokio::signal::unix::{signal, SignalKind};

        let pid = child.id().map(|id| Pid::from_raw(id as i32));
        let mut sigint = signal(SignalKind::interrupt()).ok();
        let mut sigterm = signal(SignalKind::terminate()).ok();

        loop {
            tokio::select! {
                status = child.wait() => {
                    return Ok(status?.code().unwrap_or(1));
                }
                _ = async {
                    if let Some(s) = &mut sigint { s.recv().await; }
                }, if sigint.is_some() => {
                    if let Some(p) = pid {
                        let _ = kill(p, Signal::SIGINT);
                    }
                }
                _ = async {
                    if let Some(s) = &mut sigterm { s.recv().await; }
                }, if sigterm.is_some() => {
                    if let Some(p) = pid {
                        let _ = kill(p, Signal::SIGTERM);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_c};

        let mut sig_c = ctrl_c().ok();
        let mut sig_break = ctrl_break().ok();

        loop {
            tokio::select! {
                status = child.wait() => {
                    return Ok(status?.code().unwrap_or(1));
                }
                _ = async {
                    if let Some(s) = &mut sig_c { s.recv().await; }
                }, if sig_c.is_some() => {
                    let _ = child.start_kill();
                }
                _ = async {
                    if let Some(s) = &mut sig_break { s.recv().await; }
                }, if sig_break.is_some() => {
                    let _ = child.start_kill();
                }
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(child.wait().await?.code().unwrap_or(1))
    }
}

fn map_ipc_error(err: IpcClientError) -> anyhow::Error {
    match err {
        IpcClientError::SocketNotFound { path, hint } => {
            anyhow::anyhow!("Start Argus and sign in. IPC socket not found at {path}. {hint}")
        }
        IpcClientError::Api { code, message } if code == "PROXY_DISABLED" => {
            anyhow::anyhow!("Enable Argus Proxy on this bucket in the Argus app. {message}")
        }
        other => anyhow::Error::from(other),
    }
}
