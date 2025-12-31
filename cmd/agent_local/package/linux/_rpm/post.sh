#!/usr/bin/env bash
set -euo pipefail

systemctl enable 'ak-agent.service'

systemctl restart 'ak-agent.service'

exit 0
