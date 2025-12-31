#!/usr/bin/env bash
set -euo pipefail

if [ "$1" = remove ]; then
    pam-auth-update --package --remove authentik
fi

#DEBHELPER#
