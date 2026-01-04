include ../../common.mk

TARGET := sysd
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-sysd.exe
else
BINARY_NAME := ak-sysd
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
ifeq ($(OS),Windows_NT)
	$(call go_generate_resources,authentik Platform System Service)
endif
	go build \
		${GO_BUILD_FLAGS} \
		-o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: package-gnu/linux
package-gnu/linux: build
	"$(MAKE)" -C ${TOP} browser_support/build
	mkdir -p "${TOP}/bin/${TARGET}/completions"
	"${TOP}/hack/completions.sh" \
		"${TOP}/cmd/${TARGET}" \
		"${TOP}/bin/${TARGET}/completions" \
		"${TARGET}"
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t "${TOP}/bin/${TARGET}" \
			-f "${TOP}/cmd/${TARGET}/package/linux/nfpm.yaml"
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p rpm \
			-t "${TOP}/bin/${TARGET}" \
			-f "${TOP}/cmd/${TARGET}/package/linux/nfpm.yaml"

test-deploy: package-gnu/linux
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-sysd_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ak-sysd
