include ../../common.mk

TARGET := vpkg/windows

# Package.wxs merges Microsoft's Visual C++ CRT redistributable module so a
# machine that has never had Visual Studio or a redistributable on it can still
# resolve vcruntime140.dll and vcruntime140_1.dll.
VSWHERE := C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe
VC_REDIST_MSM = $(shell "$(VSWHERE)" -latest -products '*' -property installationPath 2>/dev/null \
	| tr -d '\r' \
	| tr '\\' '/' \
	| xargs -I{} find "{}/VC/Redist/MSVC" -name 'Microsoft_VC*_CRT_x64.msm' 2>/dev/null \
	| sort -V \
	| tail -n1)

# Package.wxs embeds the Evergreen WebView2 bootstrapper and runs it when the
# runtime is missing. This is Microsoft's documented download link for it; the
# stub is versionless and always pulls the current runtime, so nothing to bump.
WEBVIEW2_BOOTSTRAPPER_URL := https://go.microsoft.com/fwlink/p/?LinkId=2124703
WEBVIEW2_BOOTSTRAPPER := ${TOP}/cache/${TARGET}/MicrosoftEdgeWebview2Setup.exe

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
	@test -n "$(VC_REDIST_MSM)" || { \
		echo "No Microsoft_VC*_CRT_x64.msm found in the Visual Studio installation vswhere reported."; \
		echo "Install the 'C++ 2022 Redistributable MSMs' component (Microsoft.VisualStudio.Component.VC.Redist.MSM)."; \
		exit 1; \
	}
	mkdir -p "${TOP}/cache/${TARGET}"
	test -f "$(WEBVIEW2_BOOTSTRAPPER)" || \
		curl --fail --silent --show-error --location --retry 3 \
			--output "$(WEBVIEW2_BOOTSTRAPPER)" \
			"$(WEBVIEW2_BOOTSTRAPPER_URL)"
	dotnet restore \
		--packages "${TOP}/cache/${TARGET}" \
		"${PWD}/"
	ROOT=${TOP} VERSION=${VERSION} \
		VC_REDIST_MSM="$(VC_REDIST_MSM)" \
		WEBVIEW2_BOOTSTRAPPER="$(WEBVIEW2_BOOTSTRAPPER)" \
		dotnet build \
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
