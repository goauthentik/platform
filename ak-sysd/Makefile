include ../common.mk

TARGET := sysd
ifeq ($(OS),Windows_NT)
BIN := ak-sysd.exe
else
BIN := ak-sysd
endif

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/cache/shared"
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/shared/release/${BIN}" "${TOP}/bin/${TARGET}/${BIND}"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}/cache/shared/release")
endif

# Kept as an alias: this used to be the escape hatch from the docker-wrapped
# build, which no longer exists.
.PHONY: build-local
build-local: build

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb
	$(TME) systemctl restart ssh
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-${TARGET}_${VERSION}+_${ARCH}.deb

.PHONY: test
test:
	$(call cargo_test,ak-sysd)
