include ../../common.mk

.ONESHELL:

OUT_TARGET := wcp

TARGETS := ak_cred_provider ak_common cefexe cefsimple

CLANG_FORMAT := "C:\Program Files\LLVM\bin\clang-format.exe"
FORMAT_FIND_ARGS := -iname '*.h' -o -iname '*.cpp' -o -iname '*.hpp'

.PHONY: bump
bump:
	sed -i 's/#define AK_VERSION ".*"/#define AK_VERSION ${VERSION}/g' ${PWD}/ak_common/include/ak_version.h

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${OUT_TARGET}"

.PHONY: lint
lint:
	$(foreach target,$(TARGETS),$(shell find "${PWD}/${target}" ${FORMAT_FIND_ARGS} | xargs ${CLANG_FORMAT} --dry-run --Werror --verbose -i))

.PHONY: lint-fix
lint-fix:
	$(foreach target,$(TARGETS),$(shell find "${PWD}/${target}" ${FORMAT_FIND_ARGS} | xargs ${CLANG_FORMAT} --verbose -i))

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p ${TOP}/bin/${OUT_TARGET}
	mkdir -p ${TOP}/cache/${OUT_TARGET}
	# To avoid the OpenSSL build using linux `link.exe`
	mv /usr/bin/link.exe /usr/bin/link_bak || true
	powershell.exe -noprofile -executionpolicy bypass -file ./build.ps1 ${PWD} ${TOP} ${OUT_TARGET}
	cp -r ${TOP}/cache/${OUT_TARGET}/ak_cred_provider/Release/* ${TOP}/bin/${OUT_TARGET}/
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/bin/${OUT_TARGET}/")
endif
