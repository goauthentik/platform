#!/bin/bash -xe
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

export PATH="/c/Program Files (x86)/GnuWin32/bin/":$PATH
export PATH="/c/Program Files/Go/bin/":$PATH
export PATH="/c/Program Files/LLVM/bin/":$PATH
eval "$($SCRIPT_DIR/vcvarsall.sh x64)"
# export PATH="/c/Strawberry/c/bin/":$PATH
which cmake
