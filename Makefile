include common.mk

.PHONY: all
all: clean gen

.PHONY: clean
clean: nss/clean pam/clean
	rm -rf ${PWD}/bin/*

.PHONY: gen
gen: gen-proto utils_rs/gen ee/psso/gen
	go generate ./...

gen-proto:
	go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
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
	go run -v ./cmd/cli ssh akadmin@authentik-platform_devcontainer-test-machine-1

test-shell:
	docker exec -it authentik-platform_devcontainer-test-machine-1 bash

test-full: clean agent/test-deploy sysd/test-deploy cli/test-deploy nss/test-deploy pam/test-deploy test-ssh

bump:
	sed -i 's/VERSION = ".*"/VERSION = "${version}"/g' common.mk
	"$(MAKE)" browser-ext/bump
	"$(MAKE)" agent/bump
	"$(MAKE)" ee/psso/bump || true

pam/%:
	"$(MAKE)" -C "${TOP}/pam" $*

nss/%:
	"$(MAKE)" -C "${TOP}/nss" $*

utils_rs/%:
	"$(MAKE)" -C "${TOP}/utils_rs" $*

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
