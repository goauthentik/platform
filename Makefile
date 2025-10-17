.PHONY: clean
LD_FLAGS = -X goauthentik.io/platform/pkg/storage.Version=${VERSION} -X goauthentik.io/platform/pkg/storage.BuildHash=dev-${VERSION_HASH}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

include common.mk
all: clean gen

clean: nss/clean pam/clean
	rm -rf ${PWD}/bin/*

gen: gen-proto utils_rs/gen ee/psso/gen
	go generate ./...

gen-proto:
	protoc \
		--go_out ${PWD} \
		--go-grpc_out=${PWD} \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**

lint: nss/lint pam/lint utils_rs/lint
	golangci-lint run

test-agent:
	go run -v ./cmd/agent_local/

test-setup:
	go run -v ./cmd/cli setup -v -a http://authentik:9000

test-ssh:
	go run -v ./cmd/cli ssh akadmin@authentik-cli_devcontainer-test-machine-1

test-shell:
	docker exec -it authentik-cli_devcontainer-test-machine-1 bash

test-full: clean agent/test-deploy sysd/test-deploy cli/test-deploy nss/test-deploy pam/test-deploy test-ssh

pam/%:
	"$(MAKE)" -C "${TOP}/pam" $*

nss/%:
	"$(MAKE)" -C "${TOP}/nss" $*

utils_rs/%:
	"$(MAKE)" -C "${TOP}/utils_rs" $*

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
