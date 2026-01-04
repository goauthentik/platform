#!/usr/bin/env bash
set -xeuo pipefail

LOG_DIR="/Library/Logs/io.goauthentik"

mkdir -p "$LOG_DIR"
chmod 755 "$LOG_DIR"
exec 2>&1 > "$LOG_DIR"/pkg_preinstall.log

INSTALL_DIR=$2
echo "$(date) Running preinstall. install_dir=${INSTALL_DIR}"

OLD_APP_LOCATION="/Applications/authentik Agent.app"

# Delete "authentik Agent.app" bundle if it exists.
if [ -d "$OLD_APP_LOCATION" ]; then
    echo "Found old app at $OLD_APP_LOCATION. Stopping and uninstalling"

    pkill "ak-agent" || echo "Unable to kill \"ak-agent\""
    sudo rm -rf "$OLD_APP_LOCATION"
    echo "Removed \"authentik Agent.app bundle\""
fi

echo "Done running preinstall script"
