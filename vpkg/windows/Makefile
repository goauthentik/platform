include ../../common.mk

TARGET := vpkg/windows
AGENT_PKG := ${TOP}/cmd/agent_local/package

.PHONY: package
package:
	mkdir -p "${TOP}/cache/${TARGET}"
	dotnet restore \
		--packages "${TOP}/cache/${TARGET}" \
		"${AGENT_PKG}/windows/"
	ROOT=${TOP} VERSION=${VERSION} VERSION_HASH_SHORT=${VERSION_HASH_SHORT} dotnet build \
		--packages "${TOP}/cache/${TARGET}" \
		--artifacts-path "${TOP}/cache/${TARGET}" \
		--output "${TOP}/bin/${TARGET}/" \
		--no-restore \
		"${AGENT_PKG}/windows/"
