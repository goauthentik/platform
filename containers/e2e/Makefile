include ../../common.mk

# Builds the instrumented packages the VM-based e2e tests (ak-platform-e2e)
# install into their base image.
.PHONY: build-packages
build-packages:
	$(MAKE) -C "${TOP}" ak-agent/build
	$(MAKE) -C "${TOP}" -j$(shell nproc) \
		ak-sysd/build \
		ak-nss/build \
		ak-pam/build \
		ak-cli/build \
		ak-browser-support/build \
		AK_GO_BUILD_FLAGS=-cover \
		RUST_BUILD_FLAGS="-C instrument-coverage"
	$(MAKE) -C "${TOP}" vpkg/linux/package
