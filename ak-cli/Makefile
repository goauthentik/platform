include ../common.mk

TARGET := cli
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak.exe
else
BINARY_NAME := ak
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/${TARGET}"
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir "${TOP}/cache/${TARGET}" \
		--verbose \
		--release
	cp "${TOP}/cache/${TARGET}/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
