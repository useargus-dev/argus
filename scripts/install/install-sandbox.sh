#!/usr/bin/env bash
# Install Argus platform sandbox redirector only.
set -euo pipefail

VERSION="${ARGUS_VERSION:-latest}"
PREFIX="${ARGUS_PREFIX:-${HOME}/.local}"
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    *) echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
if [[ "$OS" != "linux" ]]; then
  echo "install-sandbox.sh supports Linux only. Use install-sandbox.ps1 on Windows." >&2
  exit 1
fi

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

ASSET="argus-sandbox-linux-${ARCH}.tar.gz"
BASE="https://github.com/useargus/argus/releases"
if [[ "$VERSION" == "latest" ]]; then
  URL="${BASE}/latest/download/${ASSET}"
else
  URL="${BASE}/download/${VERSION}/${ASSET}"
fi

LIB_DIR="${PREFIX}/lib/argus"
echo "Installing sandbox redirector to ${LIB_DIR}"
echo "  URL: ${URL}"

if [[ "$DRY_RUN" == "1" ]]; then
  exit 0
fi

mkdir -p "$LIB_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$URL" | tar -xz -C "$TMP"
install -m 755 "$TMP/argus-redirector-linux" "$LIB_DIR/argus-redirector-linux"
echo "Installed: $LIB_DIR/argus-redirector-linux"
echo "Network capture prompts for sudo when you run 'argus run' (kernel >= 6.8)."
echo "Optional polkit policy: packaging/linux/org.argus.redirector.policy"
