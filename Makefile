include common.mk

TEST_COUNT = 1
GO_TEST_FLAGS =
TEST_OUTPUT = ${PWD}/.test-output
PROTO_OUT := "${PWD}/ak-platform/src/generated"

TARGETS := ak-pam ak-nss ak-browser-support ak-cli cmd/agent_system cmd/agent_local browser-ext ee/psso ee/wcp vpkg/macos vpkg/windows vpkg/linux containers/selenium containers/test containers/e2e

.PHONY: all
all: clean gen

.PHONY: clean
clean:
	rm -rf ${PWD}/bin/*

.PHONY: gen
gen: go-gen-proto rs-gen-proto ee/psso/gen
	go generate ./...

go-gen-proto:
	go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
	protoc \
		--go_out ${PWD} \
		--go-grpc_out=${PWD} \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**

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
		--tonic_opt=no_server \
		--prost-serde_out=$(PROTO_OUT) \
		-I $(PROTO_DIR) \
		${PROTO_DIR}/*
	cargo fmt

ci-install-deps:
ifeq ($(PLATFORM),gnu/linux)
ifeq ($(CI),true)
	sudo apt-get update
	sudo apt-get install -y \
		libpam0g-dev libudev-dev \
		libpolkit-gobject-1-dev libglib2.0-dev
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

lint-go:
	golangci-lint run

.PHONY: lint
lint: $(foreach target,$(TARGETS),${target}/lint)
	"$(MAKE)" lint-rs
	"$(MAKE)" lint-go

test:
	go tool gotest.tools/gotestsum \
		--junitfile ${PWD}/junit.xml \
		--jsonfile ${TEST_OUTPUT} \
		-- \
		-p 1 \
		-v \
		-coverprofile=${PWD}/coverage.txt \
		-covermode=atomic \
		-count=${TEST_COUNT} \
		${GO_TEST_FLAGS} \
		$(shell go list ${GO_TEST_FLAGS} ./... | grep -v goauthentik.io/platform/vnd | grep -v goauthentik.io/platform/pkg/pb)
	go tool cover \
		-html ${PWD}/coverage.txt \
		-o ${PWD}/coverage.html

test-rs: ci-install-deps
	mkdir -p "${PWD}/cache"
	cargo llvm-cov \
		--no-report \
		--ignore-filename-regex generated \
		nextest -p ${TEST_TARGET} \
			--no-tests pass
	cargo llvm-cov report \
		--codecov \
		--ignore-filename-regex generated \
		--output-path "${PWD}/cache/llvm-cov-target.json"

test-integration:
	"$(MAKE)" test GO_TEST_FLAGS=-tags=integration

test-e2e: containers/e2e/local-build
	"$(MAKE)" test GO_TEST_FLAGS=-tags=e2e

test-e2e-convert:
	go tool covdata textfmt \
		-i $(shell find ${PWD}/e2e/coverage/ -mindepth 1 -type d | xargs | sed 's/ /,/g') \
		--pkg $(shell go list ./... | grep -v goauthentik.io/platform/vnd | grep -v goauthentik.io/platform/pkg/pb | xargs | sed 's/ /,/g') \
		-o ${PWD}/coverage_in_container.txt
	go tool cover \
		-html ${PWD}/coverage_in_container.txt \
		-o ${PWD}/coverage_in_container.html

test-agent:
	go run -v ./cmd/agent_local/

test-setup:
	go run -v ./cmd/cli setup -v http://authentik:9000

test-ssh:
	go run -v ./cmd/cli ssh -i akadmin@ak-platform-test-machine

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

sysd/%:
	"$(MAKE)" -C "${TOP}/cmd/agent_system" $*

agent/%:
	"$(MAKE)" -C "${TOP}/cmd/agent_local" $*

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
