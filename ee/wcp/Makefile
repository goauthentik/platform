include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: bump
bump:
	sed -i 's/#define AK_WCP_VERSION ".*"/#define AK_WCP_VERSION ${VERSION}/g' ${PWD}/AuthentikWindowsCredentialProvider/AuthentikWindowsCredentialProvider/include/ak_version.h

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: format
format:
	find "${PWD}" -iname '*.h' -o -iname '*.cpp' | xargs clang-format -i

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	# To avoid the OpenSSL build using linux `link.exe`
	mv /usr/bin/link.exe /usr/bin/link_bak || true
	powershell.exe -noprofile -executionpolicy bypass -file ./build.ps1 ${PWD} ${TOP} ${TARGET}
	cp -r ${TOP}/cache/${TARGET}/libcef_dll_wrapper/Release/* ${TOP}/bin/${TARGET}/
ifdef SENTRY_AUTH_TOKEN
	npx getsentry/sentry-cli debug-files upload \
		--auth-token ${SENTRY_AUTH_TOKEN} \
		--include-sources \
		--org authentik-security-inc \
		--project platform \
		${TOP}/cache/${TARGET}/
endif
