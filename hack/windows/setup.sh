#!/bin/bash -xe
export PATH="/c/Program Files (x86)/GnuWin32/bin/":$PATH
export PATH="/c/Program Files/Go/bin/":$PATH
eval "$(./hack/windows/vcvarsall.sh x64)"
# export PATH="/c/Strawberry/c/bin/":$PATH
which cmake
