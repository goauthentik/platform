include ../../common.mk

TARGET := vpkg/macos
MACOS_APP_NAME := authentik Agent.app
MACOS_SIGN_CERT := 33B1F66E06FD7D8088F2301D240AE13F00399685

.PHONY: lint
lint:
	$(call lint_shellcheck,"${PWD}/scripts")

.PHONY: bump
bump:
	"${TOP}/hack/plist.py" "${PWD}/${MACOS_APP_NAME}/Contents/Info.plist" "CFBundleVersion" "${VERSION}"
	"${TOP}/hack/plist.py" "${PWD}/${MACOS_APP_NAME}/Contents/Info.plist" "CFBundleShortVersionString" "${VERSION}"

.PHONY: local
local:
	$(MAKE) -C "${TOP}" -j \
		ak-agent-desktop/build \
		sysd/build \
		ee/psso/build \
		ak-browser-support/build \
		ak-cli/build
	$(MAKE) -C "${TOP}" vpkg/macos/build-app

.PHONY: build-app
build-app:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns"
	mkdir -p "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/Resources"
	chmod +x "${TOP}/bin/cli/ak"
	chmod +x "${TOP}/bin/agent_system/ak-sysd"
	chmod +x "${TOP}/bin/agent-desktop/ak-agent-desktop"
	chmod +x "${TOP}/bin/browser_support/ak-browser-support"
	chmod +x "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/PSSO.appex/Contents/MacOS/"*
	"${TOP}/hack/completions.sh" \
		"${TOP}/bin/agent_system/ak-sysd" \
		"${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/Resources/" \
		"agent_system"
	"${TOP}/hack/completions.sh" \
		"${TOP}/bin/cli/ak" \
		"${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/Resources/" \
		"ak"
	cp -R "${PWD}/${MACOS_APP_NAME}" "${TOP}/bin/${TARGET}/"
	cp "$(shell grep -Rl io.goauthentik.platform.agent ~/Library/MobileDevice/Provisioning\ Profiles)" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/embedded.provisionprofile"
	cp "${TOP}/bin/agent-desktop/ak-agent-desktop" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/cli/ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/agent_system/ak-sysd" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/"
	cp "${TOP}/bin/browser_support/ak-browser-support" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-browser-support"
	cp -R "${TOP}/bin/psso/PSSO.appex" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/PlugIns/PSSO.appex"
	ln -s "ak" "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}/Contents/MacOS/ak-vault" || true
	codesign \
		--force \
		--verify \
		--verbose \
		--entitlements "${PWD}/authentikEndpoint.entitlements" \
		--sign ${MACOS_SIGN_CERT} \
		"${TOP}/bin/${TARGET}/${MACOS_APP_NAME}"

.PHONY: package
package: build-app
	pkgbuild \
		--component "${TOP}/bin/${TARGET}/${MACOS_APP_NAME}" \
		--identifier "io.goauthentik.agent" \
		--version "${VERSION}" \
		--scripts "${PWD}/scripts" \
		--install-location /Applications \
		"${TOP}/bin/${TARGET}/agent.pkg"
	productbuild \
		--distribution "${PWD}/distribution.xml" \
		--package-path "${TOP}/bin/${TARGET}" \
		--resources "${PWD}/resources" \
		"${TOP}/bin/${TARGET}/authentik Agent Installer.pkg"
	cd "${TOP}/bin/${TARGET}" && rm "agent.pkg" && rm -rf "${MACOS_APP_NAME}"
