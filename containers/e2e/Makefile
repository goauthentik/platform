include ../../common.mk

.PHONY: local-build
local-build:
	$(MAKE) -C ${TOP} containers/test/local-build UBUNTU_VERSION=${UBUNTU_VERSION}
	$(MAKE) -C ${TOP} containers/e2e/build-packages
	mkdir -p ${TOP}/bin
	docker build \
		-t xghcr.io/goauthentik/platform-e2e:local-${UBUNTU_VERSION} \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION_PKG}" \
		--build-arg="AK_PLATFORM_ARCH=${ARCH}" \
		--build-arg="UBUNTU_VERSION=${UBUNTU_VERSION}" \
		-f "${PWD}/Dockerfile" \
		${TOP}

.PHONY: ci-build
ci-build:
	$(MAKE) -C ${TOP} containers/test/local-build UBUNTU_VERSION=${UBUNTU_VERSION}
	mkdir -p ${TOP}/bin
	docker build \
		-t xghcr.io/goauthentik/platform-e2e:local-${UBUNTU_VERSION} \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION_PKG}" \
		--build-arg="AK_PLATFORM_ARCH=${ARCH}" \
		--build-arg="UBUNTU_VERSION=${UBUNTU_VERSION}" \
		-f "${PWD}/Dockerfile" \
		${TOP}

.PHONY: build-packages
build-packages:
	$(MAKE) -C "${TOP}" ak-agent/build
	$(MAKE) -C "${TOP}" -j$(shell nproc) \
		ak-sysd/build \
		ak-nss/build \
		ak-pam/build \
		ak-cli/build \
		ak-browser-support/build \
		AK_GO_BUILD_FLAGS=-cover \
		RUST_BUILD_FLAGS="-C instrument-coverage"
	$(MAKE) -C "${TOP}" vpkg/linux/package
