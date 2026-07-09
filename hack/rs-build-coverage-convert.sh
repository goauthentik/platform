#!/bin/bash
set -euo pipefail

PROFRAW_FILES=$(find cache/build-profraw -name '*.profraw' 2>/dev/null | tr '\n' ' ')
BUILD_SCRIPT=$(find cache/shared/release/build -path '*/ak-cli-*/build-script-build' -printf '%T@ %p\n' 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)

if [ -z "$PROFRAW_FILES" ] || [ -z "$BUILD_SCRIPT" ]; then
    echo "No Rust build-time profraw/build-script binary found, creating empty coverage file" >&2
    touch cache/rs-build-coverage.lcov
    exit 0
fi

HOST=$(rustc -vV | awk '/^host:/{print $2}')
TOOLCHAIN=$(rustup show active-toolchain | awk '{print $1}')
LLVM_DIR="$(rustup show home)/toolchains/${TOOLCHAIN}/lib/rustlib/${HOST}/bin"

"${LLVM_DIR}/llvm-profdata" merge -sparse ${PROFRAW_FILES} -o cache/rs-build.profdata
"${LLVM_DIR}/llvm-cov" export \
    -format=lcov \
    -instr-profile=cache/rs-build.profdata \
    -object "${BUILD_SCRIPT}" \
    -ignore-filename-regex='generated|\.cargo' \
    > cache/rs-build-coverage.lcov
