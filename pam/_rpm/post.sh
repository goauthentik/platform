#!/usr/bin/env bash
set -euo pipefail

function sshd_notice {
    if [ ! -f /etc/ssh/sshd_config ]; then
        return
    fi
    if ! grep -q '^KbdInteractiveAuthentication.*yes' /etc/ssh/sshd_config; then
            cat <<EOF
Because of design limitations of sshd, you need to set the following in your sshd
config file at /etc/ssh/sshd_config:

    KbdInteractiveAuthentication yes

Then reload sshd:

    sudo systemctl reload sshd
EOF
    fi
}

if [ "$1" == 1 ]; then
    mkdir -p /var/log/authentik
    # pam-auth-update --package --enable authentik
    sshd_notice
fi

exit 0
