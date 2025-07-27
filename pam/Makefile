include ../common.mk

PROTO_OUT := "${PWD}/src/generated"

build:
	cargo build --verbose

test:
	cargo test --verbose

package:
	cargo install cargo-deb
	cargo deb --verbose --deb-version ${VERSION}

test-deploy: package
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -r pam_authentik
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -i /workspaces/pam/target/debian/pam_authentik_${VERSION}_arm64.deb

gen:
	cargo install protoc-gen-prost
	cargo install protoc-gen-tonic
	mkdir -p $(PROTO_OUT)
	protoc \
	    --prost_out=$(PROTO_OUT) \
		--tonic_out=$(PROTO_OUT) \
		--tonic_opt=no_server \
		-I $(PROTO_DIR) \
		${PROTO_DIR}/*
