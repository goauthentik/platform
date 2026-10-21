#!/usr/bin/env bash
# nfpm wraps this into pacman's post_install/post_upgrade functions, so no
# `set -e` or `exit` here: the .INSTALL file is sourced.
systemctl --system daemon-reload >/dev/null || true
systemctl enable 'ak-sysd.service' >/dev/null || true
systemctl restart 'ak-sysd.service' >/dev/null || true
systemctl try-restart 'sshd.service' >/dev/null || true
# Browsers keep their native messaging host running, so they'd keep using the old
# binary until restarted. The extension reconnects by itself and starts the new one.
# Matched by path, as Linux truncates process names to 15 characters.
pkill -f '^/usr/bin/ak-browser-support' || true
