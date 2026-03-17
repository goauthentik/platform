include ../../common.mk

base_ver := $(shell cat Dockerfile | grep FROM | awk '{print $$2}' | awk -F: '{print $$2}')
container_ver := ${base_ver}-ak-${VERSION}

.PHONY: ci-container-tag
ci-container-tag:
	$(eval suffix := $(if $(strip $(AK_TAG_SUFFIX)),-pr-${AK_TAG_SUFFIX},))
	@echo ghcr.io/goauthentik/selenium:${container_ver}${suffix}

.PHONY: ci-build-args
ci-build-args:
# Only set version constraint when _not_ building a PR build
# as on a PR build, we won't have published the correct package versions
	@echo AK_PLATFORM_VER_CONSTRAINT=$(if $(strip $(AK_TAG_SUFFIX)),,=${VERSION}+ak-${VERSION_HASH_SHORT})
	@echo AK_DEPLOY_URL=${AK_DEPLOY_URL}

.PHONY: local-build
local-build:
	docker build \
		-t ghcr.io/goauthentik/selenium:${container_ver} \
		--no-cache \
		.
