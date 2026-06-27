#[cfg(target_os = "linux")]
use std::{
    env,
    ffi::OsString,
    fs,
    io::{BufRead as _, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
};

#[cfg(target_os = "linux")]
use anyhow::{anyhow, Context as _};
#[cfg(target_os = "linux")]
use cargo_metadata::{Artifact, CompilerMessage, Message, Target};

#[cfg(not(target_os = "linux"))]
fn main() {}

const EBPF_PACKAGE: &str = "mitmproxy-linux-ebpf";

/// Build mitmproxy-linux-ebpf with `build-std=core,panic_abort` (aya-build only passes `core`).
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    // Do not list mitmproxy-linux-ebpf as a build-dependency: `cargo check --all-targets`
    // would try to compile its no_std bin without nightly/build-std flags.
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../third_party/mitmproxy_rs/mitmproxy-linux-ebpf");
    build_ebpf(EBPF_PACKAGE, &root_dir)
}

#[cfg(target_os = "linux")]
fn build_ebpf(name: &str, root_dir: &PathBuf) -> anyhow::Result<()> {
    let root_dir = root_dir
        .canonicalize()
        .with_context(|| format!("ebpf crate not found at {}", root_dir.display()))?;

    let out_dir = env::var_os("OUT_DIR").ok_or(anyhow!("OUT_DIR not set"))?;
    let out_dir = PathBuf::from(out_dir);

    let endian = env::var_os("CARGO_CFG_TARGET_ENDIAN")
        .ok_or(anyhow!("CARGO_CFG_TARGET_ENDIAN not set"))?;
    let target = if endian == "big" {
        "bpfeb"
    } else if endian == "little" {
        "bpfel"
    } else {
        return Err(anyhow!("unsupported endian={endian:?}"));
    };

    const TARGET_ARCH: &str = "CARGO_CFG_TARGET_ARCH";
    let bpf_target_arch =
        env::var_os(TARGET_ARCH).ok_or_else(|| anyhow!("{TARGET_ARCH} not set"))?;
    let bpf_target_arch = bpf_target_arch
        .into_string()
        .map_err(|err| anyhow!("OsString::into_string({TARGET_ARCH}): {err:?}"))?;
    let bpf_target_arch = if bpf_target_arch.starts_with("riscv64") {
        "riscv64".to_string()
    } else {
        bpf_target_arch
    };
    let target = format!("{target}-unknown-none");

    println!("cargo:rerun-if-changed={}", root_dir.display());

    let toolchain = env::var("ARGUS_EBPF_TOOLCHAIN").unwrap_or_else(|_| "nightly".into());

    let mut cmd = Command::new("rustup");
    cmd.args([
        "run",
        &toolchain,
        "cargo",
        "build",
        "--manifest-path",
        root_dir.join("Cargo.toml").to_str().unwrap(),
        "--package",
        name,
        "-Z",
        "build-std=core,panic_abort",
        "--bins",
        "--message-format=json",
        "--release",
        "--target",
        &target,
    ]);

    const SEPARATOR: &str = "\x1f";
    let mut rustflags = OsString::new();
    for s in [
        "--cfg=bpf_target_arch=\"",
        &bpf_target_arch,
        "\"",
        SEPARATOR,
        "-Cdebuginfo=2",
        SEPARATOR,
        "-Clink-arg=--btf",
        SEPARATOR,
        "-Cpanic=abort",
    ] {
        rustflags.push(s);
    }
    cmd.env("CARGO_ENCODED_RUSTFLAGS", rustflags);

    for key in ["RUSTC", "RUSTC_WORKSPACE_WRAPPER"] {
        cmd.env_remove(key);
    }

    let target_dir = out_dir.join(name);
    cmd.arg("--target-dir").arg(&target_dir);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {cmd:?}"))?;
    let Child { stdout, stderr, .. } = &mut child;

    let stderr = stderr.take().expect("stderr");
    let stderr = BufReader::new(stderr);
    let stderr = std::thread::spawn(move || {
        for line in stderr.lines() {
            let line = line.expect("read line");
            println!("cargo:warning={line}");
        }
    });

    let stdout = stdout.take().expect("stdout");
    let stdout = BufReader::new(stdout);
    let mut executables = Vec::new();
    for message in Message::parse_stream(stdout) {
        match message.expect("valid JSON") {
            Message::CompilerArtifact(Artifact {
                executable,
                target: Target { name, .. },
                ..
            }) => {
                if let Some(executable) = executable {
                    executables.push((name, executable.into_std_path_buf()));
                }
            }
            Message::CompilerMessage(CompilerMessage { message, .. }) => {
                for line in message.rendered.unwrap_or_default().split('\n') {
                    println!("cargo:warning={line}");
                }
            }
            Message::TextLine(line) => {
                println!("cargo:warning={line}");
            }
            _ => {}
        }
    }

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for {cmd:?}"))?;
    if !status.success() {
        return Err(anyhow!("{cmd:?} failed: {status:?}"));
    }

    match stderr.join() {
        Ok(()) => {}
        Err(err) => std::panic::resume_unwind(err),
    }

    for (name, binary) in executables {
        let dst = out_dir.join(name);
        fs::copy(&binary, &dst)
            .with_context(|| format!("failed to copy {binary:?} to {dst:?}"))?;
    }

    Ok(())
}
