include ../../common.mk

TARGET := agent_local
BINARY_NAME := ak-agent

MACOS_APP_NAME := authentik Agent.app

build:
	mkdir -p ${TOP}/bin/${TARGET}
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o ${TOP}/bin/${TARGET}/${BINARY_NAME} \
		${TOP}/cmd/${TARGET}

package-macos: build
	$(MAKE) -C ${TOP} cli/build
	cp -R "${PWD}/package/macos/${MACOS_APP_NAME}" ${TOP}/bin/${TARGET}/
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS"
	cp ${TOP}/bin/${TARGET}/${BINARY_NAME} "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp ${TOP}/bin/cli/ak "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	ln -s ak "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-vault" || true
	ln -s ak "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-browser-support" || true
	pkgbuild \
		--component "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}" \
		--identifier "io.goauthentik.agent" \
		--version "${VERSION}" \
		--scripts ${PWD}/package/macos/scripts \
		"${TOP}/bin/${TARGET}/agent.pkg"

package-ubuntu: build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
