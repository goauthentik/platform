include ../common.mk

TARGET := cli
ifeq ($(OS),Windows_NT)
BIN := ak.exe
else
BIN := ak
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/${TARGET}"
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/${TARGET}/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb

.PHONY: test
test:
	$(call cargo_test,ak-cli)
