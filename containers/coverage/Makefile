include ../../common.mk

base_ver := $(shell cat Dockerfile | grep FROM | awk '{print $$2}' | awk -F: '{print $$2}')
container_ver := ${base_ver}-ak-${VERSION}
base := $(shell $(MAKE) -C ${TOP} containers/test/ci-container-tag)

.PHONY: local-build
local-build:
	$(MAKE) -C ${TOP} containers/test/local-build
	$(MAKE) -C ${TOP} containers/coverage/build-packages
	mkdir -p ${TOP}/bin
	docker build \
		-t ghcr.io/goauthentik/platform-coverage:${container_ver} \
		-t xghcr.io/goauthentik/platform-coverage:local \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION}" \
		--build-arg="AK_PLATFORM_ARCH=${ARCH}" \
		--build-arg="BASE=${base}" \
		-f "${PWD}/Dockerfile" \
		${TOP}

.PHONY: build-packages
build-packages:
	$(MAKE) -C "${TOP}" agent/package-gnu/linux AK_GO_BUILD_FLAGS=-cover
	$(MAKE) -C "${TOP}" sysd/package-gnu/linux AK_GO_BUILD_FLAGS=-cover
	$(MAKE) -C "${TOP}" cli/package-gnu/linux AK_GO_BUILD_FLAGS=-cover
	$(MAKE) -C "${TOP}" nss/package RUST_BUILD_FLAGS="-C instrument-coverage"
	$(MAKE) -C "${TOP}" pam/package RUST_BUILD_FLAGS="-C instrument-coverage"
