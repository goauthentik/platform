#!/usr/bin/env bash
set -euo pipefail

systemctl enable --user 'ak-agent.service'

systemctl restart --user 'ak-agent.service'

exit 0
