include ../../common.mk

TARGET := vpkg/windows

.PHONY: package
package:
ifeq ($(AK_IS_RELEASE),true)
	$(call sign_binary,"${TOP}/bin/agent_system/ak-sysd.exe")
	$(call sign_binary,"${TOP}/bin/agent_local/ak-agent.exe")
	$(call sign_binary,"${TOP}/bin/cli/ak.exe")
	$(call sign_binary,"${TOP}/bin/browser_support/ak-browser-support.exe")
else
	@echo "Skipping code sign for non-release build"
endif
	mkdir -p "${TOP}/cache/${TARGET}"
	dotnet restore \
		--packages "${TOP}/cache/${TARGET}" \
		"${PWD}/"
	ROOT=${TOP} VERSION=${VERSION} VERSION_HASH_SHORT=${VERSION_HASH_SHORT} dotnet build \
		--packages "${TOP}/cache/${TARGET}" \
		--artifacts-path "${TOP}/cache/${TARGET}" \
		--output "${TOP}/bin/${TARGET}/authentik Agent Installer.msi" \
		--no-restore \
		"${PWD}/"
ifeq ($(AK_IS_RELEASE),true)
	$(call sign_binary,"${TOP}/bin/${TARGET}/authentik Agent Installer.msi")
else
	@echo "Skipping code sign for non-release build"
endif
