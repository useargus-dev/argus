#!/usr/bin/env bash
# Install Argus CLI sidecar only (requires running Argus desktop for IPC).
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
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

ASSET="argus-cli-${ARCH}-unknown-${OS}-gnu.tar.gz"
if [[ "$OS" == "darwin" ]]; then
  ASSET="argus-cli-${ARCH}-apple-darwin.tar.gz"
fi

BASE="https://github.com/useargus/argus/releases"
if [[ "$VERSION" == "latest" ]]; then
  URL="${BASE}/latest/download/${ASSET}"
else
  URL="${BASE}/download/${VERSION}/${ASSET}"
fi

BIN_DIR="${PREFIX}/bin"
LIB_DIR="${PREFIX}/lib/argus"

echo "Installing Argus CLI to ${BIN_DIR}"
echo "  Release: ${VERSION}"
echo "  Asset:   ${URL}"

if [[ "$DRY_RUN" == "1" ]]; then
  exit 0
fi

mkdir -p "$BIN_DIR" "$LIB_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$URL" | tar -xz -C "$TMP"
install -m 755 "$TMP/argus-cli" "$BIN_DIR/argus"
echo "Installed: $BIN_DIR/argus"
echo "Ensure Argus desktop is running and signed in before using 'argus run'."
