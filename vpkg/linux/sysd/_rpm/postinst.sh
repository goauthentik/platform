#!/usr/bin/env bash
set -euo pipefail

if [ "$1" == 1 ]; then
    systemctl --system daemon-reload >/dev/null || true
    systemctl enable --now 'ak-sysd.service' >/dev/null || true
    systemctl restart 'ssh' >/dev/null || true
fi

# Browsers keep their native messaging host running, so they'd keep using the old
# binary until restarted. The extension reconnects by itself and starts the new one.
# Matched by path, as Linux truncates process names to 15 characters.
pkill -f '^/usr/bin/ak-browser-support' || true
