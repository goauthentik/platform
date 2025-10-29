include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: clean
clean:
	rm -rf build

.PHONY: build
build:
	${TOP}/hack/windows/setup.sh
	mkdir -p "${TOP}/bin/${TARGET}"
	cd "${TOP}/bin/${TARGET}"
	cmake -G "Visual Studio 17" "$(CURDIR)"
	cmake --build . --config Release
