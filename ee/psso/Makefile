include ../../common.mk

TARGET := psso
XCB_DEF_ARGS = -scheme PSSO \
		-project authentikPlatform.xcodeproj \
		-derivedDataPath "${TOP}/cache/${TARGET}"
XCB_EXTRA_ARGS =
SOURCES := "${PWD}/Bridge" \
		"${PWD}/PSSO"

.PHONY: clean
clean:
	xcodebuild ${XCB_DEF_ARGS} clean
	cd ${TOP} && rm -rf bin/${TARGET}

.PHONY: bump
bump:
	"${TOP}/hack/plist.py" "${PWD}/PSSO/Info.plist" "CFBundleVersion" "${VERSION}"
	"${TOP}/hack/plist.py" "${PWD}/PSSO/Info.plist" "CFBundleShortVersionString" "${VERSION}"

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	xcodebuild \
		${XCB_DEF_ARGS} \
		-configuration Release \
		-showBuildSettings \
		${XCB_EXTRA_ARGS}
	set -o pipefail && xcodebuild \
		${XCB_DEF_ARGS} \
		-configuration Release \
		CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO \
		OTHER_CODE_SIGN_FLAGS=--timestamp \
		${XCB_EXTRA_ARGS} | xcbeautify
	cp -R "${TOP}/cache/${TARGET}/Build/Products/Release/PSSO.appex" "${TOP}/bin/${TARGET}/PSSO.appex"
# tar keeps the framework symlinks and permissions, which CI artifacts drop and which break the signature
	tar -cf "${TOP}/bin/${TARGET}.tar" -C "${TOP}/bin" "${TARGET}"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/bin/${TARGET}")
endif

.PHONY: test
test:
	rm -rf "${TOP}/cache/${TARGET}/test.xcresult"
	xcodebuild \
		${XCB_DEF_ARGS} \
		test \
		-testPlan TestUnit \
		-resultBundlePath "${TOP}/cache/${TARGET}/test.xcresult" \
		-enableCodeCoverage YES \
		${XCB_EXTRA_ARGS}
	xcresultparser \
		--output-format cobertura \
		-e "${PWD}/Bridge/Generated" \
		"${TOP}/cache/${TARGET}/test.xcresult" > "${TOP}/cache/${TARGET}/coverage.xml"
	xcresultparser \
		--output-format junit \
		-e "${PWD}/Bridge/Generated" \
		"${TOP}/cache/${TARGET}/test.xcresult" > "${TOP}/cache/${TARGET}/junit.xml"

.PHONY: lint
lint:
	swift format lint \
		--configuration "${TOP}/.github/swift-format.json" \
		--recursive \
		--parallel \
		$(SOURCES)

.PHONY: lint-fix
lint-fix:
	swift format format \
		-i \
		--parallel \
		--configuration "${TOP}/.github/swift-format.json" \
		--recursive \
		$(SOURCES)

.PHONY: gen
gen:
	protoc \
		--grpc-swift-2_out=${PWD}/Bridge/Generated \
		--grpc-swift-2_opt=Server=False \
		--grpc-swift-2_opt=UseAccessLevelOnImports=True \
		--swift_out=${PWD}/Bridge/Generated \
		--swift_opt=ImplementationOnlyImports=true \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**
	$(SED_INPLACE) 's/@_implementationOnly import/internal import/g' ${PWD}/Bridge/Generated/*
