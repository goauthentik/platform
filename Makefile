include common.mk

TEST_COUNT = 1
TEST_FLAGS =
TEST_OUTPUT = ${PWD}/.test-output
PROTO_OUT := "${PWD}/src/generated"

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
	cargo clippy --fix --allow-dirty --workspace

lint-go:
	golangci-lint run

lint:
	"$(MAKE)" lint-rs
	"$(MAKE)" lint-go
	"$(MAKE)" browser-ext/lint
	"$(MAKE)" ee/psso/lint

test:
	go test \
		-p 1 \
		-v \
		-coverprofile=${PWD}/coverage.txt \
		-covermode=atomic \
		-count=${TEST_COUNT} \
		-json \
		${TEST_FLAGS} \
		$(shell go list ./... | grep -v goauthentik.io/platform/vnd | grep -v goauthentik.io/platform/pkg/pb) \
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
	$(MAKE) test TEST_FLAGS=-tags=integration

test-agent:
	go run -v ./cmd/agent_local/

test-setup:
	go run -v ./cmd/cli setup -v http://authentik:9000

test-ssh:
	go run -v ./cmd/cli ssh akadmin@authentik-platform_devcontainer-test-machine-1

test-shell:
	docker exec -it authentik-platform_devcontainer-test-machine-1 bash

test-full: clean agent/test-deploy sysd/test-deploy cli/test-deploy nss/test-deploy pam/test-deploy test-ssh

bump:
	sed -i '' 's/VERSION = ".*"/VERSION = "${version}"/g' common.mk 2>/dev/null || \
		sed -i 's/VERSION = ".*"/VERSION = "${version}"/g' common.mk
	sed -i '' 's/^version = ".*"/version = "${version}"/g' ${TOP}/Cargo.toml 2>/dev/null || \
		sed -i 's/^version = ".*"/version = "${version}"/g' ${TOP}/Cargo.toml
	sed -i '' 's/version = "[0-9]*\.[0-9]*\.[0-9]*";/version = "${version}";/' ${TOP}/flake.nix 2>/dev/null || \
		sed -i 's/version = "[0-9]*\.[0-9]*\.[0-9]*";/version = "${version}";/' ${TOP}/flake.nix
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

selenium/%:
	"$(MAKE)" -C "${TOP}/selenium" $*

# Nix targets
.PHONY: nix-build
nix-build:
	nix build .#ak-cli
	nix build .#ak-agent
	nix build .#ak-browser-support
ifeq ($(shell uname -s),Linux)
	nix build .#ak-sysd
	nix build .#libpam-authentik
	nix build .#libnss-authentik
endif

.PHONY: nix-build-go
nix-build-go:
	nix build .#ak-cli
	nix build .#ak-agent
	nix build .#ak-browser-support
ifeq ($(shell uname -s),Linux)
	nix build .#ak-sysd
endif

.PHONY: nix-develop
nix-develop:
	nix develop

.PHONY: nix-check
nix-check:
	nix flake check

.PHONY: nix-update
nix-update:
	nix flake update

.PHONY: nix-update-vendor-hash
nix-update-vendor-hash:
	@echo "Updating Go vendor hash in flake.nix..."
	@# Set vendorHash to empty string to trigger hash computation
	sed -i '' 's/vendorHash = "sha256-.*";/vendorHash = "";/' ${TOP}/flake.nix 2>/dev/null || \
		sed -i 's/vendorHash = "sha256-.*";/vendorHash = "";/' ${TOP}/flake.nix
	@# Build and capture the expected hash from the error message
	@NEW_HASH=$$(nix build .#ak-cli 2>&1 | grep -o 'got:[[:space:]]*sha256-[A-Za-z0-9+/=]*' | sed 's/got:[[:space:]]*//' | head -1) && \
	if [ -n "$$NEW_HASH" ]; then \
		sed -i '' "s/vendorHash = \"\";/vendorHash = \"$$NEW_HASH\";/" ${TOP}/flake.nix 2>/dev/null || \
			sed -i "s/vendorHash = \"\";/vendorHash = \"$$NEW_HASH\";/" ${TOP}/flake.nix; \
		echo "Updated vendorHash to: $$NEW_HASH"; \
	else \
		echo "Failed to extract hash. Check nix build output manually."; \
		exit 1; \
	fi

.PHONY: nix-cache
nix-cache:
	mkdir -p ${TOP}/bin/nix-cache/nar
	nix copy --to "file://${TOP}/bin/nix-cache" .#ak-cli
	nix copy --to "file://${TOP}/bin/nix-cache" .#ak-agent
	nix copy --to "file://${TOP}/bin/nix-cache" .#ak-browser-support
ifeq ($(shell uname -s),Linux)
	nix copy --to "file://${TOP}/bin/nix-cache" .#ak-sysd
	nix copy --to "file://${TOP}/bin/nix-cache" .#libpam-authentik
	nix copy --to "file://${TOP}/bin/nix-cache" .#libnss-authentik
endif
	echo "StoreDir: /nix/store" > ${TOP}/bin/nix-cache/nix-cache-info
	echo "WantMassQuery: 1" >> ${TOP}/bin/nix-cache/nix-cache-info
