include ../common.mk

TARGET = pam

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release
	cp "${TOP}/cache/${TARGET}/release/libauthentik_pam.so" "${TOP}/bin/${TARGET}/"

test:
	cargo test --verbose

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
