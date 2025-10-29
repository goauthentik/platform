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
	mkdir -Force "${TOP}/bin/${TARGET}"
	cd "${TOP}/bin/${TARGET}"
	cmake -G "Visual Studio 17" "$(CURDIR)"
	cmake --build . --config Release
