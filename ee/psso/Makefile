include ../../common.mk

TARGET := psso
XCB_ARGS = -scheme PSSO \
		-project authentikPlatform.xcodeproj \
		-configuration Release

.PHONY: clean
clean:
	xcodebuild clean
	cd ${TOP} && rm -rf bin/${TARGET}

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	xcodebuild ${XCB_ARGS} \
		-showBuildSettings
	xcodebuild ${XCB_ARGS} \
		-derivedDataPath "${PWD}/derivedData"
	cp -R "${PWD}/DerivedData/authentikPlatform/Build/Products/Release/PSSO.appex" "${TOP}/bin/${TARGET}/PSSO.appex"

.PHONY: format
format:
	swift format format -i --configuration ${TOP}/.github/swift-format.json --recursive ${PWD}

.PHONY: gen
gen:
	/usr/libexec/Plistbuddy -c "Set CFBundleVersion ${VERSION}" ${PWD}/PSSO/Info.plist
	/usr/libexec/Plistbuddy -c "Set CFBundleShortVersionString ${VERSION}" ${PWD}/PSSO/Info.plist
	protoc \
		--grpc-swift-2_out=${PWD}/pb_generated \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**
