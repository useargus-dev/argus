#!/usr/bin/env bash
# Install Argus CLI + platform sandbox redirector (no GUI).
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
"$DIR/install-cli.sh" "$@"
"$DIR/install-sandbox.sh" "$@"
