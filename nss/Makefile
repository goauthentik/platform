include ../common.mk

PROTO_OUT := "${PWD}/src/generated"

clean:
	cargo clean

bump:
	sed -i 's/version = ".*"/version = ${VERSION}/g' ${PWD}/Cargo.toml

build:
	cargo build --verbose --release

lint:
	cargo clippy --fix --allow-dirty

test:
	cargo test --verbose

package:
	cargo install cargo-deb
	cargo deb --verbose --deb-version ${VERSION} --variant ${ARCH}

test-deploy: package
	$(TME) dpkg -i /workspaces/nss/target/debian/libnss-authentik_${VERSION}_${ARCH}.deb
