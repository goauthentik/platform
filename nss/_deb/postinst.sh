#!/usr/bin/env bash
#DEBHELPER#
set -euo pipefail

mkdir -p /var/log/authentik

# This code was taken from authd, which got it from libnss-sss, which got it from libnss-myhostname, which got it from nss-mdns:

log() {
    echo "$*"
}

# try to insert authentik entries to the passwd, group and shadow
# lines in /etc/nsswitch.conf to automatically enable libnss-authentik
# support; do not change the configuration if the lines already
# reference some authentik lookups
insert_nss_entry() {
    log "Checking NSS setup..."
    # abort if /etc/nsswitch.conf does not exist
    if ! [ -e /etc/nsswitch.conf ]; then
        log "Could not find /etc/nsswitch.conf."
        return
    fi
    # append 'authentik' to the end of the line if it's not found already
    sed -i --regexp-extended '
      /^(passwd|group|shadow):/ {
        /\bauthentik\b/! s/$/ authentik/
      }
    ' /etc/nsswitch.conf
}

action="$1"

if [ configure = "$action" ]; then
    insert_nss_entry
fi
