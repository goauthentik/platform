include common.mk

TEST_COUNT = 1
TEST_FLAGS =
TEST_OUTPUT = ${PWD}/.test-output

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

test:
	go test \
		-p 1 \
		-v \
		-coverprofile=${PWD}/coverage.txt \
		-covermode=atomic \
		-count=${TEST_COUNT} \
		-json \
		${TEST_FLAGS} \
		$(shell go list ./... | grep -v goauthentik.io/platform/vnd) \
			2>&1 | tee ${TEST_OUTPUT}
	go tool cover \
		-html ${PWD}/coverage.txt \
		-o ${PWD}/coverage.html
	go tool github.com/jstemmer/go-junit-report/v2 -parser gojson -in ${TEST_OUTPUT} -out ${PWD}/junit.xml

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
	"$(MAKE)" nss/bump
	"$(MAKE)" pam/bump
	"$(MAKE)" ee/psso/bump || true
	"$(MAKE)" ee/wcp/bump || true

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

ee/wcp/%:
	"$(MAKE)" -C "${TOP}/ee/wcp/" $*
