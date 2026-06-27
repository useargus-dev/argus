#!/usr/bin/env bash
# Smoke tests for argus run (requires running Argus desktop + configured bucket).
set -euo pipefail

ARGUS="${ARGUS_BIN:-argus-cli}"
ENV_FILE="${ARGUS_ENV:-.env}"

echo "== argus --version =="
"$ARGUS" --version

echo "== argus status =="
"$ARGUS" status || true

echo "== argus run --dry-run =="
"$ARGUS" run --env "$ENV_FILE" --dry-run -- echo hello

echo "== IPC down error =="
if ARGUS_HOME=/nonexistent "$ARGUS" run --dry-run -- echo x 2>/dev/null; then
  echo "expected failure when misconfigured" >&2
  exit 1
fi

echo "Smoke checks completed (full curl/uvicorn tests require live bucket + sudo/admin)."
