include ../../common.mk

.ONESHELL:

SHELL := powershell.exe
.SHELLFLAGS := -NoProfile -Command
TARGET := wcp

.PHONY: clean
clean:
	rm -rf build

.PHONY: build
build:
	Remove-Item -ea 0 -Force CMakeCache.txt
	New-Item -ea 0 -Force -ItemType Directory "${TOP}/bin/${TARGET}"
	cd "${TOP}/bin/${TARGET}"
	. ${TOP}/hack/windows/setup.ps1
	nmake /P
	cmake --debug-find -G "Visual Studio 17" "$(CURDIR)"
	cmake --build . --config Release
