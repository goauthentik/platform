include ../common.mk

TARGET = nss
DEB_BUILD_GNU_TYPE := $(shell dpkg-architecture -q DEB_BUILD_GNU_TYPE)

lint:
	$(call lint_shellcheck,"${PWD}/_deb")

bump:
	sed -i 's/^version = ".*"/version = "${VERSION}"/g' ${PWD}/Cargo.toml

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release

test:
	cargo test --verbose

package: build
	DEB_BUILD_GNU_TYPE=${DEB_BUILD_GNU_TYPE} $(call nfpm_package,${TOP}/${TARGET}/nfpm.yaml)

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libnss-authentik_${VERSION}_${ARCH}.deb
