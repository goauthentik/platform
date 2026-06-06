include ../common.mk

TARGET = nss

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release
	cp "${TOP}/cache/${TARGET}/release/libauthentik_nss.so" "${TOP}/bin/${TARGET}/"

test:
	cargo test --verbose

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libnss-authentik_${VERSION}_${ARCH}.deb
