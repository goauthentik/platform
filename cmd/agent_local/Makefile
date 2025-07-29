include ../../common.mk

TARGET := agent_local
BINARY_NAME := ak-agent

build:
	mkdir -p ${TOP}/bin/${TARGET}
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o ${TOP}/bin/${TARGET}/${BINARY_NAME} \
		${TOP}/cmd/${TARGET}

.PHONY: package
package: build package-macos package-deb

package-macos:
	cp -R "${PWD}/package/macos/authentik Agent.app" ${TOP}/bin/${TARGET}/
	mkdir -p "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS"
	cp ${TOP}/bin/${TARGET}/${BINARY_NAME} "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS/"

package-deb:
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
