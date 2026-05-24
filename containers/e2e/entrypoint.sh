#!/bin/bash
dpkg-reconfigure openssh-server
exec /lib/systemd/systemd
