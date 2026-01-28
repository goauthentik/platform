include common.mk

TEST_COUNT = 1
GO_TEST_FLAGS =
TEST_OUTPUT = ${PWD}/.test-output
PROTO_OUT := "${PWD}/src/generated"

TARGETS := pam nss cmd/browser_support cmd/cli cmd/agent_system cmd/agent_local browser-ext ee/psso ee/wcp selenium

.PHONY: all
all: clean gen

.PHONY: clean
clean: nss/clean pam/clean
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
	mkdir -p $(PROTO_OUT)
	protoc \
		--prost_out=$(PROTO_OUT) \
		--prost-crate_out=$(PROTO_OUT) \
		--prost-crate_opt=no_features \
		--tonic_out=$(PROTO_OUT) \
		--tonic_opt=no_server \
		-I $(PROTO_DIR) \
		${PROTO_DIR}/*
	cargo fmt

lint-rs:
	cargo fmt --all
	cargo clippy --workspace
	cargo clippy --fix --allow-dirty --workspace

lint-go:
	golangci-lint run

.PHONY: lint
lint: $(foreach target,$(TARGETS),${target}/lint)
	"$(MAKE)" lint-rs
	"$(MAKE)" lint-go

test:
	go test \
		-p 1 \
		-v \
		-coverprofile=${PWD}/coverage.txt \
		-covermode=atomic \
		-count=${TEST_COUNT} \
		-json \
		${GO_TEST_FLAGS} \
		$(shell go list ${GO_TEST_FLAGS} ./... | grep -v goauthentik.io/platform/vnd | grep -v goauthentik.io/platform/pkg/pb) \
			2>&1 | tee ${TEST_OUTPUT}
	go tool cover \
		-html ${PWD}/coverage.txt \
		-o ${PWD}/coverage.html
	go tool github.com/jstemmer/go-junit-report/v2 \
		-parser gojson \
		-in ${TEST_OUTPUT} \
		-out ${PWD}/junit.xml \
		-set-exit-code

test-integration:
	"$(MAKE)" test GO_TEST_FLAGS=-tags=integration

test-e2e: containers/e2e/local-build
	"$(MAKE)" test GO_TEST_FLAGS=-tags=e2e
	"$(MAKE)" test-e2e-convert

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

test-full: clean agent/test-deploy sysd/test-deploy cli/test-deploy nss/test-deploy pam/test-deploy test-ssh

dev--initialize: containers/test/local-build

bump:
	sed -i 's/VERSION = ".*"/VERSION = "${version}"/g' common.mk
	sed -i 's/^version = ".*"/version = "${version}"/g' ${TOP}/Cargo.toml
	"$(MAKE)" browser-ext/bump
	"$(MAKE)" agent/bump
	"$(MAKE)" nss/bump
	"$(MAKE)" pam/bump
	"$(MAKE)" ee/psso/bump || true
	"$(MAKE)" ee/wcp/bump || true

pam/%:
	"$(MAKE)" -C "${TOP}/pam" $*

nss/%:
	"$(MAKE)" -C "${TOP}/nss" $*

browser_support/%:
	"$(MAKE)" -C "${TOP}/cmd/browser_support" $*

cli/%:
	"$(MAKE)" -C "${TOP}/cmd/cli" $*

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

containers/selenium/%:
	"$(MAKE)" -C "${TOP}/containers/selenium" $*

containers/test/%:
	"$(MAKE)" -C "${TOP}/containers/test" $*

containers/e2e/%:
	"$(MAKE)" -C "${TOP}/containers/e2e" $*
