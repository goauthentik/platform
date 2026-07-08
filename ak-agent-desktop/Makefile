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
ifeq ($(PLATFORM),gnu/linux)
ifeq ($(CI),true)
	sudo apt-get update
	sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
endif
endif
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/shared"
	pnpm install
	pnpm run build
	$(call cargo_build_local,$(TARGET),--features tauri/custom-protocol --features tauri/devtools -p ak-agent-desktop)
	cp "${TOP}/cache/shared/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
