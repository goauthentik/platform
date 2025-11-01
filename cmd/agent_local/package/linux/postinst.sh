#!/usr/bin/env bash
systemctl daemon-reload
case "$1" in
    configure)
    # creating _ak-agent group if he isn't already there
    if ! getent group _ak-agent >/dev/null; then
            addgroup --system --force-badname _ak-agent
    fi
    ;;

    abort-upgrade|abort-remove|abort-deconfigure)
    ;;

    *)
        echo "postinst called with unknown argument \`$1'" >&2
        exit 1
    ;;
esac
