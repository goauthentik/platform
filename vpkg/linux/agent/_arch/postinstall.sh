#!/usr/bin/env bash
# Sourced by pacman, see sysd/_arch/postinstall.sh.
systemctl --global enable 'ak-agent.service' >/dev/null || true
