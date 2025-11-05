include ../../common.mk

TARGET := browser_support
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-browser-support.exe
else
BINARY_NAME := ak-browser-support
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
	go build \
		${GO_BUILD_FLAGS} \
		-o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"
