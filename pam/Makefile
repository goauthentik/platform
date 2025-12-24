include ../common.mk

TARGET = pam

clean:
	cargo clean

bump:
	sed -i 's/^version = ".*"/version = ${VERSION}/g' ${PWD}/Cargo.toml

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release

test:
	cargo test --verbose

package: build
	cargo install cargo-deb
	CARGO_TARGET_DIR="${TOP}/cache/${TARGET}" cargo deb \
		--output ${TOP}/bin/${TARGET} \
		--verbose \
		--deb-version ${VERSION}

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
