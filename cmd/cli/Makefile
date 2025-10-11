include ../../common.mk

TARGET := cli
BINARY_NAME := ak

clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: package-gnu/linux
package-gnu/linux: build
	ln -s ak "${TOP}/bin/${TARGET}/ak-vault"
	ln -s ak "${TOP}/bin/${TARGET}/ak-browser-support"
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
			-f "${TOP}/cmd/${TARGET}/nfpm.yaml"

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}_${ARCH}.deb
