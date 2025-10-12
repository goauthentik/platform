include ../../common.mk

TARGET := agent_local
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-agent.exe
else
BINARY_NAME := ak-agent
endif

MACOS_APP_NAME := authentik Agent.app
MACOS_SIGN_CERT := 33B1F66E06FD7D8088F2301D240AE13F00399685
MACOS_PROVISIONING_PROFILE := $(shell grep -Rl io.goauthentik.platform.agent ~/Library/MobileDevice/Provisioning\ Profiles)

clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"
	"$(MAKE)" -C ${TOP} cli/clean
	"$(MAKE)" -C ${TOP} sysd/clean
	"$(MAKE)" -C ${TOP} ee/psso/clean || true

build:
	echo "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/bin/${TARGET}"
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: package-darwin
package-darwin: clean build
	"$(MAKE)" -C ${TOP} cli/build
	"$(MAKE)" -C ${TOP} sysd/build
	"$(MAKE)" -C ${TOP} ee/psso/build || true
	cp -R "${PWD}/package/macos/${MACOS_APP_NAME}" "${TOP}/bin/${TARGET}/"
	/usr/libexec/Plistbuddy -c "Set CFBundleVersion ${VERSION}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/Info.plist"
	/usr/libexec/Plistbuddy -c "Set CFBundleShortVersionString ${VERSION}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/Info.plist"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/"
	cp "${MACOS_PROVISIONING_PROFILE}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/embedded.provisionprofile"
	cp "${TOP}/bin/${TARGET}/${BINARY_NAME}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/cli/ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/agent_system/ak-sysd" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
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
		--scripts "${PWD}/package/macos/scripts" \
		--install-location /Applications \
		"${TOP}/bin/${TARGET}/agent.pkg"
	productbuild \
		--distribution "${PWD}/package/macos/distribution.xml" \
		--package-path "${TOP}/bin/${TARGET}" \
		--resources "${PWD}/package/macos/resources" \
		"${TOP}/bin/${TARGET}/authentik Agent Installer.pkg"
	cd "${TOP}/bin/${TARGET}" && rm "agent.pkg"
	cd "${TOP}/bin/${TARGET}" && rm -rf "${MACOS_APP_NAME}"

.PHONY: package-gnu/linux
package-gnu/linux: clean build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

.PHONY: package-msys
package-msys: clean build
	"$(MAKE)" -C ${TOP} sysd/build
	ROOT=${TOP} dotnet build \
		-o "${TOP}/bin/${TARGET}/" \
		"$(shell pwd)/package/windows/"

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
