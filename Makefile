.PHONY: clean
LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

include common.mk
all: clean gen

clean: nss/clean pam/clean
	rm -rf ${PWD}/bin/*

gen: gen-proto utils_rs/gen
	go generate ./...

gen-proto:
	go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
	protoc \
		--go_out ${PWD} \
		--go-grpc_out=${PWD} \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**

lint:
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
	$(MAKE) -C pam $*

nss/%:
	$(MAKE) -C nss $*

utils_rs/%:
	$(MAKE) -C utils_rs $*

cli/%:
	$(MAKE) -C cmd/cli $*

sysd/%:
	$(MAKE) -C cmd/agent_system $*

agent/%:
	$(MAKE) -C cmd/agent_local $*

browser-ext/%:
	$(MAKE) -C browser-ext/ $*
