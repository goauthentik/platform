include ../common.mk

TARGET = nss

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/${TARGET}/release/libnss_authentik.so" "${TOP}/bin/${TARGET}/"

.PHONY: test
test:
	$(call cargo_test,ak-nss)

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libnss-authentik_${VERSION}_${ARCH}.deb
