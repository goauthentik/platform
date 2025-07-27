.PHONY: clean bin/cli/ak bin/agent/ak-agent
LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

include common.mk
all: clean gen bin/ak bin/ak-agent

clean:
	rm -rf ${PWD}/bin/*

bin/cli/ak:
	mkdir -p ${PWD}/bin/cli
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/storage.BuildHash=${GIT_BUILD_HASH}" \
		-v -a -o ${PWD}/bin/cli/ak \
		${PWD}/cmd/cli/main
	VERSION=${VERSION} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${PWD}/bin/cli \
			-f ${PWD}/cmd/cli/main/nfpm.yaml

bin/session-manager:
	mkdir -p ${PWD}/bin/session-manager
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/storage.BuildHash=${GIT_BUILD_HASH}" \
		-v -a -o ${PWD}/bin/session-manager/aksm \
		${PWD}/session_manager
	VERSION=${VERSION} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${PWD}/bin/session-manager \
			-f ${PWD}/session_manager/package/nfpm.yaml

bin/agent/ak-agent:
	mkdir -p ${PWD}/bin/agent
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/storage.BuildHash=${GIT_BUILD_HASH}" \
		-v -a -o ${PWD}/bin/agent/ak-agent \
		${PWD}/cmd/agent
	cp -R "${PWD}/package/macos/authentik Agent.app" ${PWD}/bin/agent/
	mkdir -p "${PWD}/bin/agent/authentik Agent.app/Contents/MacOS"
	cp ${PWD}/bin/agent/ak-agent "${PWD}/bin/agent/authentik Agent.app/Contents/MacOS/"

gen:
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
	go run -v ./cmd/cli/main/ setup -v -a http://authentik:9000

test-ssh:
	go run -v ./cmd/cli/main/ ssh akadmin@authentik-cli_devcontainer-test-machine-1

pam-%:
	$(MAKE) -C pam $*

sm-%:
	$(MAKE) -C session_manager $*
