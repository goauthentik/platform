#!/usr/bin/env bash
# Sourced by pacman, see sysd/_arch/postinstall.sh.
# Append 'authentik' to the passwd, group and shadow lines unless already present.
if [ -e /etc/nsswitch.conf ]; then
    sed -i --regexp-extended '
      /^(passwd|group|shadow):/ {
        /\bauthentik\b/! s/$/ authentik/
      }
    ' /etc/nsswitch.conf
fi
