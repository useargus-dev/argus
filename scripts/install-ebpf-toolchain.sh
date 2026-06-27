#!/usr/bin/env bash
# Install Rust nightly + bpf-linker for Linux eBPF sidecars (CI and local).
set -euo pipefail

# Match mitmproxy-rs CI: rolling nightly + bpf-linker built with stable cargo.
# Do not use --profile minimal: recent nightlies ship a stub libLLVM (~43 bytes) that
# breaks bpf-linker unless the full toolchain profile is installed.
install_nightly() {
  for attempt in 1 2 3; do
    rustup toolchain uninstall nightly 2>/dev/null || true
    if rustup toolchain install nightly --profile default --component rust-src; then
      if rustc +nightly --version; then
        return 0
      fi
    fi
    echo "nightly install failed (attempt ${attempt}/3); retrying in 15s..."
    sleep 15
  done
  echo "failed to install nightly toolchain" >&2
  return 1
}

install_bpf_linker() {
  for attempt in 1 2 3; do
    # Installed with the default (stable) toolchain, same as upstream mitmproxy-rs.
    if cargo install --locked bpf-linker@0.9.15; then
      bpf-linker --version
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
