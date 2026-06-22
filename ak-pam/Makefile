include ../common.mk

TARGET = pam

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	$(call cargo_build,$(TARGET))
	cp "${TOP}/cache/${TARGET}/release/libpam_authentik.so" "${TOP}/bin/${TARGET}/"

.PHONY: test
test:
ifeq ($(PLATFORM),gnu/linux)
ifeq ($(CI),true)
	sudo apt-get update
	sudo apt-get install -y libpam0g-dev libudev-dev
endif
endif
	$(call cargo_test,ak-pam)

test-deploy: build
	$(TME) dpkg -i /workspaces/bin/${TARGET}/libpam-authentik_${VERSION}_${ARCH}.deb
	$(TME) systemctl restart ssh
