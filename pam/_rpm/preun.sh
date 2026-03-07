#!/usr/bin/env bash
set -euo pipefail

if [ $1 == 0 ]; then
    pam-auth-update --package --remove authentik
fi
