include ../common.mk

TARGET := browser_support
ifeq ($(OS),Windows_NT)
BIN := ak-browser-support.exe
else
BIN := ak-browser-support

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/${TARGET}"
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir "${TOP}/cache/${TARGET}" \
		--verbose \
		--release
	cp "${TOP}/cache/${TARGET}/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

.PHONY: test
test:
	cargo test --verbose
