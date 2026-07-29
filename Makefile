include common.mk

TEST_COUNT = 1
GO_TEST_FLAGS =
TEST_OUTPUT = ${PWD}/.test-output
PROTO_OUT := "${PWD}/ak-platform/src/generated"

TARGETS := ak-pam ak-nss ak-browser-support ak-cli ak-agent-desktop ak-agent browser-ext ee/psso ee/wcp vpkg/macos vpkg/windows vpkg/linux containers/selenium containers/test containers/e2e ak-platform ak-sysd

.PHONY: all
all: clean gen

.PHONY: clean
clean:
	rm -rf ${PWD}/bin/*

.PHONY: gen
gen: rs-gen-proto ee/psso/gen
	go generate ./...

rs-gen-proto:
	cargo install protoc-gen-prost
	cargo install protoc-gen-tonic
	cargo install protoc-gen-prost-crate
	cargo install protoc-gen-prost-serde
	mkdir -p $(PROTO_OUT)
	protoc \
		--prost_out=$(PROTO_OUT) \
		--prost_opt=compile_well_known_types \
		--prost_opt=extern_path=.google.protobuf=::pbjson_types \
		--prost-crate_out=$(PROTO_OUT) \
		--prost-crate_opt=no_features \
		--tonic_out=$(PROTO_OUT) \
		--prost-serde_out=$(PROTO_OUT) \
		-I $(PROTO_DIR) \
		${PROTO_DIR}/*
	cargo fmt --all
	cargo clippy --fix --allow-dirty -p ak-platform

ci-install-deps:
ifeq ($(PLATFORM),gnu/linux)
ifeq ($(CI),true)
	sudo apt-get update
	sudo apt-get install -y \
		libpam0g-dev libudev-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
		pkg-config libdbus-1-dev libtss2-sys1t64
endif
endif

lint-rs:
	cargo fmt --all
	cargo clippy --workspace \
		${RS_TEST_FLAGS}
	cargo clippy --fix \
		--allow-dirty \
		--workspace \
		${RS_TEST_FLAGS}

.PHONY: lint
lint: $(foreach target,$(TARGETS),${target}/lint)
	"$(MAKE)" lint-rs

test-integration:
	"$(MAKE)" test GO_TEST_FLAGS=-tags=integration

test-e2e: containers/e2e/local-build
	$(call cargo_test,ak-platform-e2e)

test-e2e-ci:
	$(call cargo_test,ak-platform-e2e)

test-e2e-convert:
	$(call rs_e2e_coverage_convert)

test-setup:
	go run -v ./cmd/cli setup -v http://authentik:9000

test-ssh:
	ssh -i akadmin@ak-platform-test-machine

test-shell:
	docker exec -it authentik-platform_devcontainer-test-machine-1 bash

test-join:
	docker exec \
		-it \
		--env AK_SYS_INSECURE_ENV_TOKEN=test-enroll-key \
		authentik-platform_devcontainer-test-machine-1 \
		ak-sysd domains join ak -a http://authentik:9000

test-full: clean agent/test-deploy sysd/test-deploy ak-cli/test-deployak-nss/test-deployak-pam/test-deploy test-ssh

dev--initialize: containers/test/local-build

bump:
	sed -i 's/VERSION = .*/VERSION = ${version}/g' common.mk
	sed -i 's/^version = "${VERSION}"/version = "${version}"/g' ${TOP}/Cargo.toml ${TOP}/Cargo.lock
	"$(MAKE)" browser-ext/bump
	"$(MAKE)" vpkg/macos/bump
	"$(MAKE)" ee/psso/bump || true
	"$(MAKE)" ee/wcp/bump || true

ak-pam/%:
	"$(MAKE)" -C "${TOP}/ak-pam" $*

ak-nss/%:
	"$(MAKE)" -C "${TOP}/ak-nss" $*

ak-browser-support/%:
	"$(MAKE)" -C "${TOP}/ak-browser-support" $*

ak-cli/%:
	"$(MAKE)" -C "${TOP}/ak-cli" $*

ak-platform/%:
	"$(MAKE)" -C "${TOP}/ak-platform" $*

ak-sysd/%:
	"$(MAKE)" -C "${TOP}/ak-sysd" $*

ak-agent/%:
	"$(MAKE)" -C "${TOP}/ak-agent" $*

ak-agent-desktop/%:
	"$(MAKE)" -C "${TOP}/ak-agent-desktop" $*

ak-api-cli-gen/%:
	"$(MAKE)" -C "${TOP}/ak-api-cli-gen" $*

ak-platform-facts/%:
	"$(MAKE)" -C "${TOP}/ak-platform-facts" $*

browser-ext/%:
	"$(MAKE)" -C "${TOP}/browser-ext/" $*

ee/psso/%:
	"$(MAKE)" -C "${TOP}/ee/psso/" $*

ee/wcp/%:
	"$(MAKE)" -C "${TOP}/ee/wcp/" $*

vpkg/macos/%:
	"$(MAKE)" -C "${TOP}/vpkg/macos" $*

vpkg/windows/%:
	"$(MAKE)" -C "${TOP}/vpkg/windows" $*

vpkg/linux/%:
	"$(MAKE)" -C "${TOP}/vpkg/linux" $*

containers/builder/%:
	"$(MAKE)" -C "${TOP}/containers/builder" $*

containers/selenium/%:
	"$(MAKE)" -C "${TOP}/containers/selenium" $*

containers/test/%:
	"$(MAKE)" -C "${TOP}/containers/test" $*

containers/e2e/%:
	"$(MAKE)" -C "${TOP}/containers/e2e" $*
