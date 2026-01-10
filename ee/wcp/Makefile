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
	find "${PWD}/ak_cred_provider" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --dry-run --Werror --verbose -i
	find "${PWD}/cefexe" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --dry-run --Werror --verbose -i
	find "${PWD}/cefsimple" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --dry-run --Werror --verbose -i

.PHONY: lint-fix
lint-fix:
	find "${PWD}/ak_cred_provider" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --verbose -i
	find "${PWD}/cefexe" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --verbose -i
	find "${PWD}/cefsimple" -iname '*.h' -o -iname '*.cpp' | xargs clang-format.exe --verbose -i

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	# To avoid the OpenSSL build using linux `link.exe`
	mv /usr/bin/link.exe /usr/bin/link_bak || true
	powershell.exe -noprofile -executionpolicy bypass -file ./build.ps1 ${PWD} ${TOP} ${TARGET}
	cp -r ${TOP}/cache/${TARGET}/ak_cred_provider/Release/* ${TOP}/bin/${TARGET}/
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/bin/${TARGET}/")
endif
