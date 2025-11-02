include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: bump
bump:
	sed -i 's/#define AK_WCP_VERSION ".*"/#define AK_WCP_VERSION ${VERSION}/g' ${PWD}/AuthentikWindowsCredentialProvider/AuthentikWindowsCredentialProvider/include/Version.h

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	# To avoid the OpenSSL build using linux `link.exe`
	mv /usr/bin/link.exe /usr/bin/link_bak || true
	powershell.exe ./build.ps1 ${PWD} ${TOP} ${TARGET}
	cp -r ${TOP}/cache/${TARGET}/libcef_dll_wrapper/Release/* ${TOP}/bin/${TARGET}/
