include ../common.mk

TARGET = nss

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/shared
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/shared/release/libnss_authentik.so" "${TOP}/bin/${TARGET}/"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/cache/shared/release")
endif

.PHONY: test
test:
	$(call cargo_test,ak-nss)

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libnss-authentik_${VERSION}_${ARCH}.deb
