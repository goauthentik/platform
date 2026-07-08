include ../common.mk

TARGET := browser_support
ifeq ($(OS),Windows_NT)
BIN := ak-browser-support.exe
else
BIN := ak-browser-support
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/shared"
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/shared/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

.PHONY: test
test:
	$(call cargo_test,ak-browser-support)
