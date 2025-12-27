include ../common.mk

base_ver := $(shell cat Dockerfile | grep FROM | awk '{print $$2}' | awk -F: '{print $$2}')
container_ver := ${base_ver}-ak-${VERSION}

.PHONY: ci-container-tag
ci-container-tag:
	@echo ghcr.io/goauthentik/selenium:${container_ver}${AK_TAG_SUFFIX}

.PHONY: ci-build-args
ci-build-args:
	@echo AK_PLATFORM_VER=${VERSION}
	@echo AK_DEPLOY_URL=${AK_DEPLOY_URL}

.PHONY: local-build
local-build:
	docker build \
		-t ghcr.io/goauthentik/selenium:${container_ver} \
		--no-cache \
		--build-arg="AK_PLATFORM_VER=${VERSION}" \
		.
