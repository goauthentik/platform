#!/bin/bash
set -euo pipefail

# Usage: rs-coverage-convert.sh <profraw_dir> <output_basename> [object ...]
#
# Each trailing "object" is either a plain path to an instrumented binary/library
# (included if it exists), or "newest:<search_dir>:<path_pattern>" to resolve to
# the most-recently-modified file under <search_dir> matching <path_pattern>
# (used to find Cargo's hashed build-script-build output).

PROFRAW_DIR="$1"
OUTPUT_BASENAME="$2"
shift 2

mkdir -p "${PWD}/cache"

PROFRAW_FILES=$(find "$PROFRAW_DIR" -name '*.profraw' 2>/dev/null | tr '\n' ' ')

if [ -z "$PROFRAW_FILES" ]; then
    echo "No Rust profraw files found in ${PROFRAW_DIR}, creating empty coverage file"
    touch "${PWD}/cache/${OUTPUT_BASENAME}-coverage.lcov"
    exit 0
fi

OBJECTS=""
for arg in "$@"; do
    case "$arg" in
        newest:*)
            spec="${arg#newest:}"
            search_dir="${spec%%:*}"
            path_pattern="${spec#*:}"
            resolved=$(find "$search_dir" -path "$path_pattern" -printf '%T@ %p\n' 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)
            if [ -n "$resolved" ]; then OBJECTS="$OBJECTS -object $resolved"; fi
            ;;
        *)
            if [ -f "$arg" ]; then OBJECTS="$OBJECTS -object $arg"; fi
            ;;
    esac
done

if [ -z "$OBJECTS" ]; then
    echo "No instrumented Rust binaries found, creating empty coverage file"
    touch "${PWD}/cache/${OUTPUT_BASENAME}-coverage.lcov"
    exit 0
fi

HOST=$(rustc -vV 2>/dev/null | awk '/^host:/{print $2}')
TOOLCHAIN=$(rustup show active-toolchain 2>/dev/null | awk '{print $1}')
LLVM_DIR="$(rustup show home 2>/dev/null)/toolchains/${TOOLCHAIN}/lib/rustlib/${HOST}/bin"

"${LLVM_DIR}/llvm-profdata" merge -sparse ${PROFRAW_FILES} \
    -o "${PWD}/cache/${OUTPUT_BASENAME}-merged.profdata"

"${LLVM_DIR}/llvm-cov" export \
    -format=lcov \
    -instr-profile="${PWD}/cache/${OUTPUT_BASENAME}-merged.profdata" \
    ${OBJECTS} \
    -ignore-filename-regex='generated|\.cargo' \
    > "${PWD}/cache/${OUTPUT_BASENAME}-coverage.lcov"
