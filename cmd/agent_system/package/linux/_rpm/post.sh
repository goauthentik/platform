#!/usr/bin/env bash
set -euo pipefail

systemctl enable 'ak-sysd.service'

if [ $1 -eq 1 ] ; then
    # creating _ak-agent group if he isn't already there
    if ! getent group _ak-agent >/dev/null; then
            addgroup --system --force-badname _ak-agent
    fi
fi

systemctl restart 'ak-sysd.service'

exit 0
