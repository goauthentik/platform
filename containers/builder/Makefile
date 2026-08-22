include ../../common.mk

# The image only depends on its own definition and the pinned toolchain, so tag
# it by content: CI can then skip the build entirely whenever the tag already
# exists in the registry, and a toolchain bump republishes within the same PR.
# `git hash-object` avoids branching on shasum (macOS) vs sha256sum (Linux).
container_ver := $(shell cat "${PWD}/Dockerfile" "${TOP}/rust-toolchain.toml" | git hash-object --stdin | head -c 12)

.PHONY: ci-container-tag
ci-container-tag:
	@echo ghcr.io/goauthentik/platform-builder:${container_ver}

.PHONY: ci-build-args
ci-build-args:
	@echo RUST_TOOLCHAIN=$(shell sed -n 's/channel = "\(.*\)"/\1/p' ${TOP}/rust-toolchain.toml)

.PHONY: local-build
local-build:
	docker build \
		-t ghcr.io/goauthentik/platform-builder:${container_ver} \
		-f "${PWD}/Dockerfile" \
		"${TOP}"
