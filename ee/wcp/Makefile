include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: clean
clean:
	rm -rf build

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	# To avoid the OpenSSL build using linux `link.exe`
	mv /usr/bin/link.exe /usr/bin/link_bak || true
	VS170COMNTOOLS=${VS170COMNTOOLS} powershell.exe ./build.ps1 ${PWD} ${TOP} ${TARGET}
	cp -r ${TOP}/cache/${TARGET}/libcef_dll_wrapper/Release/* ${TOP}/bin/${TARGET}/
