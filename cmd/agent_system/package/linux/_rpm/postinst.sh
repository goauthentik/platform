#!/usr/bin/env bash
set -euo pipefail

if [ "$1" == 1 ]; then
    systemctl --system daemon-reload >/dev/null || true
    systemctl enable --now 'ak-sysd.service' >/dev/null || true
fi
