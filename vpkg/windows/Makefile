include ../../common.mk

TARGET := vpkg/windows

.PHONY: local
local:
	$(MAKE) -C "${TOP}" -j \
		ak-sysd/build \
		ak-agent-desktop/build \
		ak-cli/build \
		ak-browser-support/build \
		ee/wcp/build
	$(MAKE) -C "${TOP}" vpkg/windows/package

.PHONY: package
package:
ifeq ($(AK_IS_RELEASE),true)
	$(call sign_binary,"${TOP}/bin/sysd/ak-sysd.exe")
	$(call sign_binary,"${TOP}/bin/agent-desktop/ak-agent-desktop.exe")
	$(call sign_binary,"${TOP}/bin/cli/ak.exe")
	$(call sign_binary,"${TOP}/bin/browser_support/ak-browser-support.exe")
else
	@echo "Skipping code sign for non-release build"
endif
	mkdir -p "${TOP}/cache/${TARGET}"
	dotnet restore \
		--packages "${TOP}/cache/${TARGET}" \
		"${PWD}/"
	ROOT=${TOP} VERSION=${VERSION} dotnet build \
		--packages "${TOP}/cache/${TARGET}" \
		--artifacts-path "${TOP}/cache/${TARGET}" \
		--output "${TOP}/bin/${TARGET}" \
		--no-restore \
		"${PWD}/"
ifeq ($(AK_IS_RELEASE),true)
	$(call sign_binary,"${TOP}/bin/${TARGET}/authentik Agent Installer.msi")
else
	@echo "Skipping code sign for non-release build"
endif
