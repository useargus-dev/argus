#[cfg(target_os = "linux")]
use std::{
    env,
    ffi::OsString,
    fs,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "linux")]
use anyhow::{anyhow, Context as _};

#[cfg(not(target_os = "linux"))]
fn main() {}

const EBPF_BIN: &str = "mitmproxy-linux";

/// Build mitmproxy-linux-ebpf with `build-std=core,panic_abort`.
/// Do not list mitmproxy-linux-ebpf as a build-dependency: `cargo check --all-targets`
/// would try to compile its no_std bin without nightly/build-std flags.
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../third_party/mitmproxy_rs/mitmproxy-linux-ebpf");
    build_ebpf(EBPF_BIN, &root_dir)
}

#[cfg(target_os = "linux")]
fn build_ebpf(bin_name: &str, root_dir: &Path) -> anyhow::Result<()> {
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
    let target_triple = format!("{target}-unknown-none");

    println!("cargo:rerun-if-changed={}", root_dir.display());

    let toolchain = env::var("ARGUS_EBPF_TOOLCHAIN").unwrap_or_else(|_| "nightly".into());
    let target_dir = out_dir.join("ebpf-target");

    let mut cmd = Command::new("rustup");
    cmd.args([
        "run",
        &toolchain,
        "cargo",
        "build",
        "--manifest-path",
        root_dir.join("Cargo.toml").to_str().unwrap(),
        "--package",
        "mitmproxy-linux-ebpf",
        "-Z",
        "build-std=core,panic_abort",
        "--bins",
        "--release",
        "--target",
        &target_triple,
        "--target-dir",
        target_dir.to_str().unwrap(),
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

    cmd.stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {cmd:?}"))?;

    let stderr = child.stderr.take().expect("stderr");
    let stderr = BufReader::new(stderr);
    for line in stderr.lines() {
        let line = line.expect("read line");
        println!("cargo:warning={line}");
    }

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for {cmd:?}"))?;
    if !status.success() {
        return Err(anyhow!("{cmd:?} failed: {status:?}"));
    }

    let binary = target_dir
        .join(&target_triple)
        .join("release")
        .join(bin_name);
    let dst = out_dir.join(bin_name);
    fs::copy(&binary, &dst)
        .with_context(|| format!("failed to copy {} to {}", binary.display(), dst.display()))?;

    Ok(())
}
