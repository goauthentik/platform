include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: bump
bump:
	sed -i 's/#define AK_WCP_VERSION ".*"/#define AK_WCP_VERSION ${VERSION}/g' ${PWD}/ak_cred_provider/include/ak_version.h

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: lint
lint:
	find "${PWD}" -iname '*.h' -o -iname '*.cpp' | xargs clang-format -i

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	powershell.exe -noprofile -executionpolicy bypass -file ./build.ps1 ${PWD} ${TOP} ${TARGET}
	cp -r ${TOP}/cache/${TARGET}/ak_cred_provider/Release/* ${TOP}/bin/${TARGET}/
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/bin/${TARGET}/")
endif
