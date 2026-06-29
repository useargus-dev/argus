#!/bin/sh
# Post-install for deb/rpm: expose `argus` CLI on PATH and set ARGUS_HOME for shells.
set -e

ARGUS_HOME="/usr/lib/argus"
CLI="${ARGUS_HOME}/lib/argus/argus-cli"
REDIR="${ARGUS_HOME}/lib/argus/argus-redirector-linux"

if [ ! -x "$CLI" ]; then
  echo "argus postinst: missing bundled CLI at $CLI" >&2
  exit 1
fi

if [ ! -x "$REDIR" ]; then
  echo "argus postinst: missing bundled redirector at $REDIR" >&2
  exit 1
fi

mkdir -p /usr/local/bin
ln -sf "$CLI" /usr/local/bin/argus

cat > /etc/profile.d/argus.sh <<EOF
# Argus install paths (package postinst)
export ARGUS_HOME="${ARGUS_HOME}"
EOF
chmod 644 /etc/profile.d/argus.sh
