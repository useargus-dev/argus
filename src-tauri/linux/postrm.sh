#!/bin/sh
# Post-remove for deb/rpm: drop CLI symlink and profile snippet.
set -e

rm -f /usr/local/bin/argus
rm -f /etc/profile.d/argus.sh
