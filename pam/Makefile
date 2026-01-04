include ../common.mk

TARGET = pam
DEB_BUILD_ARCH := $(shell dpkg-architecture -q DEB_BUILD_ARCH)
DEB_BUILD_GNU_TYPE := $(shell dpkg-architecture -q DEB_BUILD_GNU_TYPE)

clean:
	cargo clean

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
	VERSION=${VERSION} ARCH=${DEB_BUILD_ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/${TARGET}/nfpm.yaml
	VERSION=${VERSION} ARCH=${DEB_BUILD_ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p rpm \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/${TARGET}/nfpm.yaml

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
