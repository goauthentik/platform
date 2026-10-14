include ../../common.mk

.PHONY: build
build:
	docker build \
		-t ${DOCKER_BUILDER_IMAGE} \
		-f "${PWD}/Dockerfile" \
		"${TOP}"
