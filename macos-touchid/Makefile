include ../common.mk

build:
	swift build -c release --product TouchIDLibrary
	swiftc -emit-library \
		-emit-clang-header-path $(PWD)/lib/TouchIDLibrary.h \
		-o $(PWD)/lib/libTouchIDLibrary.dylib \
		${PWD}/Sources/TouchIDLibrary/*.swift \
		-framework LocalAuthentication \
		-framework LocalAuthenticationEmbeddedUI \
		-framework AppKit \
		-framework Foundation
