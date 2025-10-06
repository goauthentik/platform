include ../common.mk

PROTO_OUT := "${PWD}/src/generated"

clean:
	cargo clean

build:
	cargo build --verbose --release

lint:
	cargo clippy --fix --allow-dirty

test:
	cargo test --verbose

package:
	cargo install cargo-deb
	cargo deb --verbose --deb-version ${VERSION}

test-deploy: package
	$(TME) dpkg -i /workspaces/pam/target/debian/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
