#!/usr/bin/env bash
#DEBHELPER#
set -euo pipefail

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

# Reload AppArmor so it picks up the drop-in we ship under /etc/apparmor.d.
# The apparmor package registers no dpkg trigger on that directory, so this only
# shortcuts the wait until the next apparmor reload or reboot; the rules
# themselves are picked up either way. Reload the whole policy rather than a
# single profile: the abstractions/nameservice.d drop-in is included by every
# profile that performs user/group lookups.
#
# Expected to no-op inside build containers: apparmor_parser is usually absent,
# and even when present /sys/kernel/security is not mounted during a container
# build, so no policy can be loaded. Note that AppArmor policy is host-global,
# so a container never confines its own processes -- the host's profiles do.
reload_apparmor() {
    log "Checking AppArmor setup..."
    if ! command -v apparmor_parser >/dev/null 2>&1; then
        log "apparmor_parser not found, skipping."
        return
    fi
    if ! aa-enabled --quiet 2>/dev/null; then
        log "AppArmor is not enabled, skipping."
        return
    fi
    if systemctl reload apparmor.service >/dev/null 2>&1; then
        return
    fi
    log "Could not reload apparmor.service, falling back to unix-chkpwd only."
    if ! [ -f /etc/apparmor.d/unix-chkpwd ]; then
        log "Could not find /etc/apparmor.d/unix-chkpwd."
        return
    fi
    apparmor_parser -r -T -W /etc/apparmor.d/unix-chkpwd \
        || log "Failed to reload the unix-chkpwd profile, changes apply after reboot."
}

action="$1"

if [ configure = "$action" ]; then
    insert_nss_entry
    reload_apparmor
fi
