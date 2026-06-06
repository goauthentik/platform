include ../common.mk

TARGET := browser_support

bump:
	sed -i 's/^version = ".*"/version = "${VERSION}"/g' ${PWD}/Cargo.toml

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release

test:
	cargo test --verbose
