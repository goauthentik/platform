#!/bin/bash
set -euo pipefail
files=$(find /build/bin -type f -name '*deb' | grep ${AK_PLATFORM_ARCH})
for file in $files; do
    dpkg -i $file
done
