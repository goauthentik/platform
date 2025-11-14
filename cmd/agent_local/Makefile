include ../../common.mk

TARGET := agent_local
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-agent.exe
else
BINARY_NAME := ak-agent
endif

MACOS_APP_NAME := authentik Agent.app
MACOS_SIGN_CERT := 33B1F66E06FD7D8088F2301D240AE13F00399685

clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"
	"$(MAKE)" -C ${TOP} cli/clean
	"$(MAKE)" -C ${TOP} sysd/clean
	"$(MAKE)" -C ${TOP} browser_support/clean
	"$(MAKE)" -C ${TOP} ee/wcp/clean || true
	"$(MAKE)" -C ${TOP} ee/psso/clean || true

build:
	mkdir -p "${TOP}/bin/${TARGET}"
ifeq ($(OS),Windows_NT)
	$(call go_generate_resources,authentik Platform Agent)
endif
	go build \
		${GO_BUILD_FLAGS} \
		-o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: bump
bump:
	/usr/libexec/Plistbuddy -c "Set CFBundleVersion ${VERSION}" "${PWD}/package/macos/${MACOS_APP_NAME}/Contents/Info.plist"
	/usr/libexec/Plistbuddy -c "Set CFBundleShortVersionString ${VERSION}" "${PWD}/package/macos/${MACOS_APP_NAME}/Contents/Info.plist"

.PHONY: package-darwin
package-darwin: clean build
	"$(MAKE)" -C ${TOP} cli/build
	"$(MAKE)" -C ${TOP} sysd/build
	"$(MAKE)" -C ${TOP} browser_support/build
	"$(MAKE)" -C ${TOP} ee/psso/build || true
	cp -R "${PWD}/package/macos/${MACOS_APP_NAME}" "${TOP}/bin/${TARGET}/"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/"
	cp "$(shell grep -Rl io.goauthentik.platform.agent ~/Library/MobileDevice/Provisioning\ Profiles)" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/embedded.provisionprofile"
	cp "${TOP}/bin/${TARGET}/${BINARY_NAME}" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/cli/ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/agent_system/ak-sysd" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/browser_support/ak-browser-support" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-browser-support"
	ln -s "ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-vault" || true
	cp -R "${TOP}/bin/psso/PSSO.appex" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/PSSO.appex" || true
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
	"$(MAKE)" -C ${TOP} browser_support/build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/linux/nfpm.yaml

.PHONY: package-msys
package-msys: clean
# build with -H=windowsgui to run as a true systray application
	"$(MAKE)" -C ${TOP} agent/build LD_FLAGS=-H=windowsgui
	"$(MAKE)" -C ${TOP} cli/build
	"$(MAKE)" -C ${TOP} sysd/build
	"$(MAKE)" -C ${TOP} browser_support/build
	"$(MAKE)" -C ${TOP} ee/wcp/build || true
	mkdir -p "${TOP}/cache/${TARGET}"
	dotnet restore \
		--packages "${TOP}/cache/${TARGET}" \
		"$(shell pwd)/package/windows/"
	ROOT=${TOP} VERSION=${VERSION} dotnet build \
		--packages "${TOP}/cache/${TARGET}" \
		--artifacts-path "${TOP}/cache/${TARGET}" \
		--output "${TOP}/bin/${TARGET}/" \
		--no-restore \
		"$(shell pwd)/package/windows/"

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
