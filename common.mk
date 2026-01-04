.SHELLFLAGS += -x -e
SHELL = /bin/bash
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = "0.35.5"
VERSION_HASH = $(shell git rev-parse HEAD)
ifeq ($(OS),Windows_NT)
ARCH := $(PROCESSOR_ARCHITEW6432)
else
ARCH := $(shell dpkg-architecture -q DEB_BUILD_ARCH)
endif
PLATFORM := $(shell bash -c "uname -o | tr '[:upper:]' '[:lower:]'")

TOP = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
PROTO_DIR := "${TOP}/protobuf"

_LD_FLAGS = ${LD_FLAGS} -X goauthentik.io/platform/pkg/meta.Version=${VERSION} -X goauthentik.io/platform/pkg/meta.BuildHash=dev-${VERSION_HASH}
GO_BUILD_FLAGS = -ldflags "${_LD_FLAGS}" -v
RUST_BUILD_FLAGS =

TME := docker exec authentik-platform_devcontainer-test-machine-1

define sentry_upload_symbols
	npx @sentry/cli debug-files upload \
		--auth-token ${SENTRY_AUTH_TOKEN} \
		--include-sources \
		--org authentik-security-inc \
		--project platform \
		$(1)
endef

define go_generate_resources
	go tool goversioninfo \
		-icon="${TOP}/cmd/agent_local/package/windows/resources/icon.ico" \
		-company="Authentik Security Inc." \
		-copyright="2025 Authentik Security Inc." \
		-file-version=${VERSION} \
		-product-version=${VERSION} \
		-comment="$(1)" \
		-description="$(1)" \
		-product-name="$(1)" \
		-skip-versioninfo
endef
