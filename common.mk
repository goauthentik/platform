.SHELLFLAGS += -x -e
SHELL = /bin/bash
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = 0.50.2
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

RUST_BUILD_FLAGS ?=
DOCKER_BUILDER_IMAGE ?= authentik/ak-builder
CARGO_CRATE_DIR := $(subst $(TOP),,$(CURDIR))

ifneq ($(LOCAL_WORKSPACE),)
CONTAINER_TOP := ${LOCAL_WORKSPACE}
else
CONTAINER_TOP := ${TOP}
endif

define cargo_build_local
RUSTFLAGS="$(RUST_BUILD_FLAGS)" \
		AK_VERSION=${VERSION} \
		AK_BUILDHASH=${VERSION_HASH} \
		AK_TAG=${VERSION_TAG} \
		cargo build \
		--target-dir $(TOP)cache/shared \
		--verbose \
		--release $(2)
endef

ifeq ($(PLATFORM),gnu/linux)
define cargo_build
	rm -rf "$(CONTAINER_TOP)cache/build-profraw"
	mkdir -p "$(CONTAINER_TOP)cache/build-profraw"
	docker run --rm \
		-i \
		--volume "$(CONTAINER_TOP):/workspace" \
		--workdir "/workspace/$(CARGO_CRATE_DIR)" \
		--env RUSTFLAGS="$(RUST_BUILD_FLAGS)" \
		--env AK_VERSION="${VERSION}" \
		--env AK_BUILDHASH="${VERSION_HASH}" \
		--env AK_TAG="${VERSION_TAG}" \
		--env LLVM_PROFILE_FILE="/workspace/cache/build-profraw/%m_%p.profraw" \
		$(DOCKER_BUILDER_IMAGE) \
		cargo build \
			--target-dir /workspace/cache/shared \
			--verbose \
			--release
endef
else
define cargo_build
RUSTFLAGS="$(RUST_BUILD_FLAGS)" \
		AK_VERSION=${VERSION} \
		AK_BUILDHASH=${VERSION_HASH} \
		AK_TAG=${VERSION_TAG} \
		cargo build \
		--target-dir $(TOP)cache/shared \
		--verbose \
		--release $(2)
endef
endif

define cargo_test
	mkdir -p "${PWD}/cache"
	cargo llvm-cov \
		--no-report \
		--ignore-filename-regex generated \
		nextest -p $(1) \
			--no-tests pass \
			--no-fail-fast \
			--test-threads 1
	cargo llvm-cov report \
		--codecov \
		--ignore-filename-regex generated \
		--output-path "${PWD}/cache/llvm-cov-target.json"
	cargo llvm-cov report \
		--html \
		--ignore-filename-regex generated \
		--output-dir "${PWD}/cache/llvm-cov-html/"
endef

define rs_e2e_coverage_convert
	mkdir -p "${PWD}/cache"
	PROFRAW_FILES=$$(find "${PWD}/ak-platform-e2e/coverage/rs" -name '*.profraw' 2>/dev/null | tr '\n' ' '); \
	if [ -z "$$PROFRAW_FILES" ]; then \
		echo "No Rust profraw files found in ak-platform-e2e/coverage/rs, creating empty coverage file"; \
		touch "${PWD}/cache/rs-e2e-coverage.lcov"; \
	else \
		HOST=$$(rustc -vV 2>/dev/null | awk '/^host:/{print $$2}'); \
		TOOLCHAIN=$$(rustup show active-toolchain 2>/dev/null | awk '{print $$1}'); \
		LLVM_DIR=$$(rustup show home 2>/dev/null)/toolchains/$$TOOLCHAIN/lib/rustlib/$$HOST/bin; \
		$$LLVM_DIR/llvm-profdata merge -sparse $$PROFRAW_FILES \
			-o "${PWD}/cache/rs-e2e-merged.profdata"; \
		OBJECTS=""; \
		for bin in "${PWD}/bin/cli/ak" "${PWD}/bin/agent/ak-agent" \
				"${PWD}/bin/nss/libnss_authentik.so" "${PWD}/bin/pam/libpam_authentik.so"; do \
			if [ -f "$$bin" ]; then OBJECTS="$$OBJECTS -object $$bin"; fi; \
		done; \
		if [ -z "$$OBJECTS" ]; then \
			echo "No instrumented Rust binaries found in bin/, creating empty coverage file"; \
			touch "${PWD}/cache/rs-e2e-coverage.lcov"; \
		else \
			$$LLVM_DIR/llvm-cov export \
				-format=lcov \
				-instr-profile="${PWD}/cache/rs-e2e-merged.profdata" \
				$$OBJECTS \
				-ignore-filename-regex='generated|\.cargo' \
				> "${PWD}/cache/rs-e2e-coverage.lcov"; \
		fi; \
	fi
endef

define rs_build_coverage_convert
	mkdir -p "${PWD}/cache"
	docker run --rm \
		-i \
		--volume "$(CONTAINER_TOP):/workspace" \
		--workdir /workspace \
		$(DOCKER_BUILDER_IMAGE) \
		bash hack/rs-build-coverage-convert.sh
endef

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
			-t ${TOP}/bin/$(2) \
			-f $(1)
	VERSION_PKG=${VERSION_PKG} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p rpm \
			-t ${TOP}/bin/$(2) \
			-f $(1)
endef

define _target_template
.PHONY: $(1)/%
$(1)/%:
	"$(MAKE)" -C "${TOP}/$(1)" $$*
endef
