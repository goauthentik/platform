.PHONY: clean bin/cli/ak bin/agent/ak-agent bin/pam/pam_authentik.so
.ONESHELL:
.SHELLFLAGS += -x -e
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = "0.1.0"
LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v
LOCAL_BUILD_ARCH := linux/amd64

all: clean gen bin/ak bin/ak-agent bin/pam/pam_authentik.so

clean:
	rm -rf ${PWD}/bin/*
	rm -rf ${PWD}/*.h

bin/cli/ak:
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-$(shell git rev-parse HEAD))
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
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-$(shell git rev-parse HEAD))
	mkdir -p ${PWD}/bin/agent
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/storage.BuildHash=${GIT_BUILD_HASH}" \
		-v -a -o ${PWD}/bin/agent/ak-agent \
		${PWD}/cmd/agent
	cp -R "${PWD}/package/macos/authentik Agent.app" ${PWD}/bin/agent/
	mkdir -p "${PWD}/bin/agent/authentik Agent.app/Contents/MacOS"
	cp ${PWD}/bin/agent/ak-agent "${PWD}/bin/agent/authentik Agent.app/Contents/MacOS/"

bin/pam/pam_authentik.so: .
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-$(shell git rev-parse HEAD))
	mkdir -p ${PWD}/bin/pam
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/storage.BuildHash=${GIT_BUILD_HASH}" \
		-v -buildmode=c-shared -o bin/pam/pam_authentik.so ${PWD}/cmd/pam/
	VERSION=${VERSION} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${PWD}/bin/pam \
			-f ${PWD}/cmd/pam/nfpm.yaml

pam-docker: clean gen
	cd ${PWD}/hack/pam/local_build && docker build \
		--platform $(LOCAL_BUILD_ARCH) \
		--tag pam_authentik:local_build \
		.
	docker run \
		-it \
		--platform $(LOCAL_BUILD_ARCH) \
		--rm \
		-v ${PWD}:/data \
		-v pam_authentik-go-cache:/root/go/pkg \
		-v pam_authentik-go-build-cache:/root/.cache \
		pam_authentik:local_build \
		make bin/pam/deb

gen:
	go generate ./...

test-install: bin/cli/ak bin/pam/pam_authentik.so
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -i /workspaces/bin/pam/pam_authentik_${VERSION}_arm64.deb
	sudo dpkg -i ${PWD}/bin/cli/authentik-cli_${VERSION}_arm64.deb

test-setup:
	ak setup -v -a http://authentik:9000

test-ssh:
	go run -v ./cmd/cli/main/ ssh akadmin@authentik-cli_devcontainer-test-machine-1

foo:
	cd pam && cargo build
	docker exec authentik-cli_devcontainer-test-machine-1 cp /workspaces/pam/target/debug/libpam_authentik.so /usr/lib/security/libpam_authentik.so
	docker exec authentik-cli_devcontainer-test-machine-1 cp /workspaces/pam/target/debug/libpam_authentik.so /usr/lib64/security/libpam_authentik.s
