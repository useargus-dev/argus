#!/usr/bin/env bash
# Install Rust nightly + bpf-linker for Linux eBPF sidecars (CI and local).
set -euo pipefail

# Pin nightly to avoid corrupt rolling-nightlies on GHA (libLLVM*.so: file too short).
export ARGUS_EBPF_TOOLCHAIN="${ARGUS_EBPF_TOOLCHAIN:-nightly-2026-04-01}"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
TOOLCHAIN_DIR="${HOME}/.rustup/toolchains/${ARGUS_EBPF_TOOLCHAIN}-${HOST_TRIPLE}"

verify_nightly_llvm() {
  local llvm_so
  llvm_so="$(find "${TOOLCHAIN_DIR}" -name 'libLLVM-*-rust-*.so' 2>/dev/null | head -1 || true)"
  if [[ -z "${llvm_so}" ]]; then
    echo "no libLLVM shared library found in ${TOOLCHAIN_DIR}" >&2
    return 1
  fi
  local size
  size="$(stat -c%s "${llvm_so}" 2>/dev/null || stat -f%z "${llvm_so}")"
  if [[ "${size}" -lt 1000000 ]]; then
    echo "corrupt LLVM (${llvm_so}, ${size} bytes)" >&2
    return 1
  fi
  echo "verified ${llvm_so} (${size} bytes)"
}

install_nightly() {
  for attempt in 1 2 3; do
    rustup toolchain uninstall "${ARGUS_EBPF_TOOLCHAIN}" 2>/dev/null || true
    if ! rustup toolchain install "${ARGUS_EBPF_TOOLCHAIN}" --profile minimal --component rust-src; then
      echo "nightly install failed (attempt ${attempt}/3); retrying in 15s..."
      sleep 15
      continue
    fi
    if ! rustc +"${ARGUS_EBPF_TOOLCHAIN}" --version >/dev/null 2>&1; then
      echo "nightly unusable (attempt ${attempt}/3); retrying in 15s..."
      sleep 15
      continue
    fi
    if verify_nightly_llvm; then
      return 0
    fi
    echo "nightly LLVM corrupt (attempt ${attempt}/3); retrying in 15s..."
    sleep 15
  done
  echo "failed to install a working nightly toolchain" >&2
  return 1
}

install_bpf_linker() {
  for attempt in 1 2 3; do
    if cargo +"${ARGUS_EBPF_TOOLCHAIN}" install --locked bpf-linker@0.9.15; then
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
