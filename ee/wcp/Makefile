include ../../common.mk

.ONESHELL:

OUT_TARGET := wcp
# These crates are members of the root workspace, so every cargo invocation
# here runs from the workspace root and selects them by package name.
WCP_PACKAGES := -p ak-ee-wcp-wire -p ak-ee-wcp -p ak-ee-wcp-cef-host -p ak-ee-wcp-e2e
WCP_BUILD_PACKAGES := -p ak-ee-wcp -p ak-ee-wcp-cef-host
WCP_MANIFEST := "${TOP}Cargo.toml"
WCP_TARGET_DIR := "${TOP}cache/wcp"
CEF_PIN := $(shell grep -A1 '^name = "cef-dll-sys"' ${TOP}Cargo.lock | grep version | sed -E 's/.*"([0-9.]+)\+.*/\1/')
CEF_PATH := $(TOP)cache/cef

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${OUT_TARGET}" "cache/wcp"

.PHONY: cef-runtime
cef-runtime:
	cargo install export-cef-dir --version "${CEF_PIN}" --locked
	export-cef-dir --force "${CEF_PATH}"

.PHONY: build
build: cef-runtime
	mkdir -p "${TOP}bin/${OUT_TARGET}"
	CEF_PATH="${CEF_PATH}" \
		RUSTFLAGS="$(RUST_BUILD_FLAGS)" \
		AK_VERSION=${VERSION} \
		AK_BUILDHASH=${VERSION_HASH} \
		AK_TAG=${VERSION_TAG} \
		cargo build --release \
			--target-dir ${WCP_TARGET_DIR} \
			--manifest-path ${WCP_MANIFEST} \
			${WCP_BUILD_PACKAGES}
	cp "${TOP}cache/wcp/release/ak_cred_provider.dll" "${TOP}bin/${OUT_TARGET}/"
	cp "${TOP}cache/wcp/release/ak_cred_provider.pdb" "${TOP}bin/${OUT_TARGET}/" || true
	cp "${TOP}cache/wcp/release/ak_cred_provider.dll.lib" "${TOP}bin/${OUT_TARGET}/ak_cred_provider.lib"
	cp "${TOP}cache/wcp/release/ak_cef.exe" "${TOP}bin/${OUT_TARGET}/"
	cp "${TOP}cache/wcp/release/ak_cef.pdb" "${TOP}bin/${OUT_TARGET}/" || true
	cp "${CEF_PATH}"/*.dll "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}"/*.pak "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/icudtl.dat" "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/v8_context_snapshot.bin" "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/vk_swiftshader_icd.json" "${TOP}bin/${OUT_TARGET}/"
	cp -r "${CEF_PATH}/locales" "${TOP}bin/${OUT_TARGET}/"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}bin/${OUT_TARGET}/")
endif

# Coverage uses llvm-cov's manual (show-env) flow rather than
# `$(call cargo_test,...)`, because the e2e tests LoadLibraryW the real
# ak_cred_provider.dll and spawn the real ak_cef.exe: `cargo test`/nextest on
# their own only ever build test harnesses, never the cdylib and bin
# artifacts. Exporting llvm-cov's env first lets an explicit `cargo build`
# place both -- instrumented, so their coverage counts -- in the same
# directory the test binary resolves them from.
# AK_WCP_E2E opts in to the process-level tests: a CI runner has no real
# ak-sysd bound to the pipe and is elevated enough to seed the HKLM
# capability cache. See e2e/README.md for what that turns on.
#
# One chained shell command on purpose: `.ONESHELL:` is a no-op on the GNU
# Make 3.81 that ships in GnuWin32, so `cd`/`export`/`source` would not carry
# across separate recipe lines.
.PHONY: test
test: cef-runtime
	mkdir -p "${TOP}cache"
	cd "${TOP}" && \
		export CEF_PATH="${CEF_PATH}" && \
		export AK_WCP_E2E=1 && \
		source <(cargo llvm-cov show-env --export-prefix) && \
		cargo llvm-cov clean --workspace && \
		cargo build ${WCP_BUILD_PACKAGES} && \
		cargo nextest run ${WCP_PACKAGES} --no-tests pass --no-fail-fast --test-threads 1 && \
		cargo llvm-cov report --codecov --ignore-filename-regex generated \
			--output-path "${TOP}cache/llvm-cov-target.json" && \
		cargo llvm-cov report --html --ignore-filename-regex generated \
			--output-dir "${TOP}cache/llvm-cov-html/"

.PHONY: lint
lint:
	cargo fmt --manifest-path ${WCP_MANIFEST} ${WCP_PACKAGES}
	CEF_PATH="${CEF_PATH}" cargo clippy \
		--target-dir ${WCP_TARGET_DIR} \
		--manifest-path ${WCP_MANIFEST} \
		${WCP_PACKAGES}
	CEF_PATH="${CEF_PATH}" cargo clippy --fix --allow-dirty \
		--target-dir ${WCP_TARGET_DIR} \
		--manifest-path ${WCP_MANIFEST} \
		${WCP_PACKAGES}
