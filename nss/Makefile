include ../common.mk

TARGET = nss
DEB_BUILD_GNU_TYPE := $(shell dpkg-architecture -q DEB_BUILD_GNU_TYPE)

lint:
	$(call lint_shellcheck,"_deb")
	$(call lint_shellcheck,"_rpm")

bump:
	sed -i 's/^version = ".*"/version = ${VERSION}/g' ${PWD}/Cargo.toml

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
	VERSION=${VERSION} ARCH=${ARCH} DEB_BUILD_GNU_TYPE=${DEB_BUILD_GNU_TYPE} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/${TARGET}/nfpm.yaml
	VERSION=${VERSION} ARCH=${ARCH} DEB_BUILD_GNU_TYPE=${DEB_BUILD_GNU_TYPE} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p rpm \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/${TARGET}/nfpm.yaml

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libnss-authentik_${VERSION}_${ARCH}.deb
