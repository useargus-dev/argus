#!/usr/bin/env bash
# Package Windows sandbox redirector + WinDivert for GitHub releases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
STAGE="$DIST/sandbox-win-staging"
RELEASE="$ROOT/target/release"

mkdir -p "$STAGE"
cp "$RELEASE/argus-redirector-windows.exe" "$STAGE/"

for f in WinDivert.dll WinDivert64.sys; do
  if [[ -f "$RELEASE/$f" ]]; then
    cp "$RELEASE/$f" "$STAGE/"
  elif [[ -f "$ROOT/third_party/windivert/$f" ]]; then
    cp "$ROOT/third_party/windivert/$f" "$STAGE/"
  else
    echo "warning: $f not found — run scripts/stage-windivert.ps1 on Windows after building redirector" >&2
  fi
done

mkdir -p "$DIST"
(
  cd "$STAGE"
  if command -v zip >/dev/null 2>&1; then
    zip -r "$DIST/argus-sandbox-windows-x86_64.zip" .
  else
    powershell.exe -NoProfile -Command "Compress-Archive -Path '$STAGE/*' -DestinationPath '$DIST/argus-sandbox-windows-x86_64.zip' -Force"
  fi
)
echo "Created $DIST/argus-sandbox-windows-x86_64.zip"
