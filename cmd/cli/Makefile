include ../../common.mk

TARGET := cli
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak.exe
else
BINARY_NAME := ak
endif

clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
ifeq ($(OS),Windows_NT)
	$(call go_generate_resources,authentik Platform CLI)
endif
	go build \
		${GO_BUILD_FLAGS} \
		-o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: package-gnu/linux
package-gnu/linux:
	$(call nfpm_package,${TOP}/cmd/${TARGET}/package/linux/nfpm.yaml)

test-deploy: package-gnu/linux
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
