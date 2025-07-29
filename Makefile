.PHONY: clean
LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

include common.mk
all: clean gen

clean:
	rm -rf ${PWD}/bin/*

gen: gen-proto pam/gen
	go generate ./...

gen-proto:
	go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
	protoc \
		--go_out ${PWD} \
		--go-grpc_out=${PWD} \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**

test-setup:
	go run -v ./cmd/cli setup -v -a http://authentik:9000

test-ssh:
	go run -v ./cmd/cli ssh akadmin@authentik-cli_devcontainer-test-machine-1

test-shell:
	docker exec -it authentik-cli_devcontainer-test-machine-1 bash

test-full: clean agent/test-deploy sys/test-deploy cli/test-deploy pam/test-deploy test-ssh

pam/%:
	$(MAKE) -C pam $*

cli/%:
	$(MAKE) -C cmd/cli $*

sys/%:
	$(MAKE) -C cmd/agent_system $*

agent/%:
	$(MAKE) -C cmd/agent_local $*
