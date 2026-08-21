include ../common.mk

TARGET := sysd-ctrl-relay
ifeq ($(OS),Windows_NT)
BIN := ak-sysd-ctrl-relay.exe
else
BIN := ak-sysd-ctrl-relay
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/shared"
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/shared/release/${BIN}" "${TOP}/bin/${TARGET}/${BIN}"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/cache/shared/release")
endif

build-local:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/shared"
	$(call cargo_build_local,$(TARGET))
	cp "${TOP}/cache/shared/release/${BIN}" "${TOP}/bin/${TARGET}/${BIN}"

.PHONY: test
test:
	$(call cargo_test,ak-sysd-ctrl-relay)
