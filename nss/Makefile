include ../common.mk

PROTO_OUT := "${PWD}/src/generated"

clean:
	cargo clean

build:
	cargo build --verbose

test:
	cargo test --verbose

package:
	cargo install cargo-deb
	cargo deb --verbose --deb-version ${VERSION}

test-deploy: package
	$(TME) dpkg -i /workspaces/nss/target/debian/libnss-authentik_${VERSION}_${ARCH}.deb

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
	cargo fmt
