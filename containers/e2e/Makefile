include ../../common.mk

.PHONY: local-build
local-build:
	$(MAKE) -C ${TOP} containers/test/local-build
	$(MAKE) -C ${TOP} containers/e2e/build-packages
	mkdir -p ${TOP}/bin
	docker build \
		-t xghcr.io/goauthentik/platform-e2e:local \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION}" \
		--build-arg="AK_PLATFORM_ARCH=${ARCH}" \
		-f "${PWD}/Dockerfile" \
		${TOP}

.PHONY: build-packages
build-packages:
	$(MAKE) -C "${TOP}" agent/package-gnu/linux AK_GO_BUILD_FLAGS=-cover
	$(MAKE) -C "${TOP}" -j$(shell nproc) \
		sysd/package-gnu/linux \
		cli/package-gnu/linux \
		nss/package \
		pam/package \
		AK_GO_BUILD_FLAGS=-cover \
		RUST_BUILD_FLAGS="-C instrument-coverage"
