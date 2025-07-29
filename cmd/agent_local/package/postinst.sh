#!/usr/bin/env bash
systemctl daemon-reload
systemctl --global --now enable ak-agent
