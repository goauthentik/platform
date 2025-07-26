.PHONY: clean bin/cli/ak bin/agent/ak-agent
.SHELLFLAGS += -x -e
LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

include common.mk
all: clean gen bin/ak bin/ak-agent

clean:
	rm -rf ${PWD}/bin/*

bin/cli/ak:
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH})
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

bin/agent/ak-agent:
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH})
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

test-setup:
	ak setup -v -a http://authentik:9000

test-ssh:
	go run -v ./cmd/cli/main/ ssh akadmin@authentik-cli_devcontainer-test-machine-1

pam-%:
	$(MAKE) -C pam $*
