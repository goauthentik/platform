#!/usr/bin/env bash
systemctl daemon-reload
systemctl --now enable ak-agent
