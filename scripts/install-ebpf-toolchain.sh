#!/usr/bin/env bash
# Install Rust nightly + bpf-linker for Linux eBPF sidecars (CI and local).
set -euo pipefail

install_nightly() {
  for attempt in 1 2 3; do
    rustup toolchain uninstall nightly 2>/dev/null || true
    rustup toolchain install nightly --profile minimal --component rust-src
    if rustc +nightly --version >/dev/null 2>&1; then
      return 0
    fi
    echo "nightly install failed (attempt ${attempt}/3); retrying in 15s..."
    sleep 15
  done
  echo "failed to install a working nightly toolchain" >&2
  return 1
}

install_bpf_linker() {
  for attempt in 1 2 3; do
    if cargo install --locked bpf-linker@0.9.15; then
      return 0
    fi
    echo "bpf-linker install failed (attempt ${attempt}/3); retrying in 15s..."
    sleep 15
  done
  echo "failed to install bpf-linker" >&2
  return 1
}

install_nightly
install_bpf_linker
