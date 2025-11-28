#!/usr/bin/env bash
systemctl daemon-reload || true
systemctl enable --now ak-sysd || true
