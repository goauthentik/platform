include ../../common.mk

TARGET := wcp

.PHONY: clean
clean:


.PHONY: bump
bump:


.PHONY: build
build:
	mkdir -p "${PWD}/build"
	cd "${PWD}/build"
	cmake -G "Visual Studio 17" -A x64 "${PWD}"
	cmake --build "${PWD}/build" --config Release
	gc cef.sln
	devenv cef.sln /build Release
