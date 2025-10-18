include ../../common.mk

TARGET := psso
XCB_ARGS = -scheme PSSO \
		-project authentikPlatform.xcodeproj \
		-configuration Release

.PHONY: clean
clean:
	xcodebuild clean
	cd ${TOP} && rm -rf bin/${TARGET}

.PHONY: bump
bump:
	/usr/libexec/Plistbuddy -c "Set CFBundleVersion ${VERSION}" ${PWD}/PSSO/Info.plist
	/usr/libexec/Plistbuddy -c "Set CFBundleShortVersionString ${VERSION}" ${PWD}/PSSO/Info.plist

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	xcodebuild ${XCB_ARGS} \
		-showBuildSettings
	xcodebuild ${XCB_ARGS} \
		-derivedDataPath "${PWD}/DerivedData"
	cp -R "${PWD}/DerivedData/Build/Products/Release/PSSO.appex" "${TOP}/bin/${TARGET}/PSSO.appex"
ifdef SENTRY_AUTH_TOKEN
	npx getsentry/sentry-cli debug-files upload \
		--auth-token ${SENTRY_AUTH_TOKEN} \
		--include-sources \
		--org authentik-security-inc \
		--project platform \
		${PWD}/DerivedData/Build/Products/Release
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
