include ../common.mk

TARGET := agent-desktop
ifeq ($(OS),Windows_NT)
BIN := ak-agent-desktop.exe
else
BIN := ak-agent-desktop
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/${TARGET}"
	npm i
	npm run build
	$(call cargo_build,$(TARGET),--features tauri/custom-protocol)
	cp "${TOP}/cache/${TARGET}/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
