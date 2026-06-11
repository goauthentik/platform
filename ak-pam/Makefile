include ../common.mk

TARGET = pam

build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/${TARGET}/release/libpam_authentik.so" "${TOP}/bin/${TARGET}/"

test:
	cargo test --verbose

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh

ci-install-deps:
ifeq ($(PLATFORM),gnu/linux)
ifeq ($(CI),true)
	sudo apt-get update
	sudo apt-get install -y libpam0g-dev libudev-dev
endif
endif
