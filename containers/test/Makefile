include ../../common.mk

base_ver := $(shell cat Dockerfile | grep FROM | awk '{print $$2}' | awk -F: '{print $$2}')
container_ver := ${base_ver}-ak-${VERSION}

.PHONY: ci-container-tag
ci-container-tag:
	$(eval suffix := $(if $(strip $(AK_TAG_SUFFIX)),-pr-${AK_TAG_SUFFIX},))
	@echo ghcr.io/goauthentik/platform-test:${container_ver}${suffix}

.PHONY: ci-build-args
ci-build-args:
	@echo AK_PLATFORM_VER=${VERSION}
	@echo AK_DEPLOY_URL=${AK_DEPLOY_URL}

.PHONY: local-build
local-build:
	mkdir -p ${TOP}/bin
	docker build \
		-t ghcr.io/goauthentik/platform-test:${container_ver} \
		-t xghcr.io/goauthentik/platform-test:local \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION}" \
		--build-arg="AK_PLATFORM_ARCH=${ARCH}" \
		-f "${PWD}/Dockerfile" \
		.
