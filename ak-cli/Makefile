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
	RUSTFLAGS="${RUST_BUILD_FLAGS}" cargo build \
		--target-dir "${TOP}/cache/${TARGET}" \
		--verbose \
		--release
	cp "${TOP}/cache/${TARGET}/release/${BIN}" "${TOP}/bin/${TARGET}/${BIN}"
	mkdir -p "${TOP}/bin/${TARGET}/man"
	"${TOP}/cache/${TARGET}/release/${BIN}" generate-manpage "${TOP}/bin/${TARGET}/man"
	mkdir -p "${TOP}/bin/${TARGET}/completions"
	for s in bash zsh fish; do \
		"${TOP}/cache/${TARGET}/release/${BIN}" generate-completions $$s > "${TOP}/bin/${TARGET}/completions/ak.$$s"; \
	done

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
