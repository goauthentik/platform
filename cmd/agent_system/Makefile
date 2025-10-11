include ../../common.mk

TARGET := agent_system
BINARY_NAME := ak-sysd

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: package-linux
package-linux: build
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
			-f "${TOP}/cmd/${TARGET}/package/nfpm.yaml"

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-sysd_${VERSION}_${ARCH}.deb
	$(TME) bash -c 'echo -n "this-token-is-for-testing-dont-use" | ak-sysd domains join default -a http://authentik:9000'
	$(TME) systemctl restart ak-sysd
