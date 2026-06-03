.SHELLFLAGS += -x -e
SHELL = /bin/bash
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = 0.43.2
VERSION_HASH = $(shell git rev-parse HEAD)
VERSION_TAG = $(shell git tag --points-at HEAD)
ifeq ($(CI),true)
	ifeq ($(AK_IS_RELEASE),true)
		VERSION_PKG = ${VERSION}
	else
		VERSION_PKG = ${VERSION}+ak-${shell git rev-parse HEAD | head -c 8}
	endif
else
	VERSION_PKG = ${VERSION}
endif
VERSION_TS = $(shell date +%s)
PLATFORM := $(shell bash -c "uname -o | tr '[:upper:]' '[:lower:]'")
ifeq ($(OS),Windows_NT)
ARCH := $(PROCESSOR_ARCHITEW6432)
else ifeq ($(PLATFORM),gnu/linux)
ARCH := $(shell dpkg-architecture -q DEB_BUILD_ARCH)
else
ARCH := $(shell uname -p)
endif

TOP = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
PROTO_DIR := "${TOP}/protobuf"

_LD_FLAGS = ${LD_FLAGS} \
	-X goauthentik.io/platform/pkg/meta.Version=${VERSION} \
	-X goauthentik.io/platform/pkg/meta.BuildHash=${VERSION_HASH} \
	-X goauthentik.io/platform/pkg/meta.Tag=${VERSION_TAG}
GO_BUILD_FLAGS = -ldflags "${_LD_FLAGS}" -v ${AK_GO_BUILD_FLAGS}
RUST_BUILD_FLAGS =

TME := docker exec authentik-platform_devcontainer-test-machine-1

define lint_shellcheck
	find $(1) -type f -name '*.sh'  -exec "shellcheck" "--format=gcc" {} \;
endef

define sentry_upload_symbols
	npx @sentry/cli debug-files upload \
		--auth-token ${SENTRY_AUTH_TOKEN} \
		--include-sources \
		--org authentik-security-inc \
		--project platform \
		$(1)
endef

define sign_binary
	smctl keypair ls
	smctl sign \
		--keypair-alias=key_1504090127 \
		--simple \
		--verbose \
		--input $(1)
endef

define go_generate_resources
	go tool goversioninfo \
		-icon="${TOP}/vpkg/windows/resources/icon.ico" \
		-company="Authentik Security Inc." \
		-copyright="2025 Authentik Security Inc." \
		-file-version=${VERSION} \
		-product-version=${VERSION} \
		-comment="$(1)" \
		-description="$(1)" \
		-product-name="$(1)" \
		-skip-versioninfo \
		-64
endef

define nfpm_package
	VERSION_PKG=${VERSION_PKG} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f $(1)
	VERSION_PKG=${VERSION_PKG} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p rpm \
			-t ${TOP}/bin/${TARGET} \
			-f $(1)
endef

define _target_template
.PHONY: $(1)/%
$(1)/%:
	"$(MAKE)" -C "${TOP}/$(1)" $$*
endef
