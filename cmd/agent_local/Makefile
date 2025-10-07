include ../../common.mk

TARGET := agent_local
BINARY_NAME := ak-agent

MACOS_APP_NAME := authentik Agent.app
MACOS_SIGN_CERT := 33B1F66E06FD7D8088F2301D240AE13F00399685
MACOS_PROVISIONING_PROFILE := $(shell grep -Rl io.goauthentik.platform.agent ~/Library/MobileDevice/Provisioning\ Profiles)

build:
	mkdir -p ${TOP}/bin/${TARGET}
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o ${TOP}/bin/${TARGET}/${BINARY_NAME} \
		${TOP}/cmd/${TARGET}

clean:
	cd ${TOP} && rm -rf bin/${TARGET}
	$(MAKE) -C ${TOP} cli/clean
	$(MAKE) -C ${TOP} ee/psso/clean || true

package-darwin: clean build
	$(MAKE) -C ${TOP} cli/build
	$(MAKE) -C ${TOP} ee/psso/build || true
	cp -R "${PWD}/package/macos/${MACOS_APP_NAME}" "${TOP}/bin/${TARGET}/"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/"
	cp "${MACOS_PROVISIONING_PROFILE}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/embedded.provisionprofile"
	cp "${TOP}/bin/${TARGET}/${BINARY_NAME}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/cli/ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	ln -s "ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-vault" || true
	ln -s "ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-browser-support" || true
	cp -R "${TOP}/bin/psso/PSSO.appex" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/PSSO.appex" || true
	@echo "## Siging with entitlements"
	codesign \
		--force \
		--verify \
		--verbose \
		--entitlements "${PWD}/package/macos/authentikEndpoint.entitlements" \
		--sign ${MACOS_SIGN_CERT} \
		"${TOP}/bin/${TARGET}/${MACOS_APP_NAME}"
	pkgbuild \
		--component "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}" \
		--identifier "io.goauthentik.agent" \
		--version "${VERSION}" \
		--scripts ${PWD}/package/macos/scripts \
		--install-location /Applications \
		"${TOP}/bin/${TARGET}/agent.pkg"
	productbuild \
		--distribution "${PWD}/package/macos/distribution.xml" \
		--package-path "${TOP}/bin/${TARGET}" \
		--resources "${PWD}/package/macos/resources" \
		"${TOP}/bin/${TARGET}/authentik Agent Installer.pkg"
	rm "${TOP}/bin/${TARGET}/agent.pkg"

package-linux: build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
