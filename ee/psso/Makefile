include ../../common.mk

TARGET := psso
XCB_ARGS = -scheme PSSO \
		-project authentikPlatform.xcodeproj \
		-configuration Release \
		-derivedDataPath "${TOP}/cache/${TARGET}"

.PHONY: clean
clean:
	xcodebuild ${XCB_ARGS} clean
	cd ${TOP} && rm -rf bin/${TARGET}

.PHONY: bump
bump:
	/usr/libexec/Plistbuddy -c "Set CFBundleVersion ${VERSION}" ${PWD}/PSSO/Info.plist
	/usr/libexec/Plistbuddy -c "Set CFBundleShortVersionString ${VERSION}" ${PWD}/PSSO/Info.plist

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	xcodebuild ${XCB_ARGS} \
		-showBuildSettings
	xcodebuild ${XCB_ARGS}
	cp -R "${TOP}/cache/${TARGET}/Build/Products/Release/PSSO.appex" "${TOP}/bin/${TARGET}/PSSO.appex"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/bin/${TARGET}")
endif

.PHONY: format
format:
	swift format format \
		-i \
		--configuration "${TOP}/.github/swift-format.json" \
		--recursive \
		${PWD}/Generated \
		${PWD}/PSSO

.PHONY: gen
gen:
	protoc \
		--grpc-swift-2_out=${PWD}/Generated \
		--grpc-swift-2_opt=Server=False \
		--grpc-swift-2_opt=UseAccessLevelOnImports=True \
		--swift_out=${PWD}/Generated \
		--swift_opt=ImplementationOnlyImports=true \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**
	sed -i 's/@_implementationOnly import/internal import/g' ${PWD}/Generated/*
