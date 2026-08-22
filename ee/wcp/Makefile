include ../../common.mk

.ONESHELL:

OUT_TARGET := wcp
# These crates are members of the root workspace, so every cargo invocation
# here runs from the workspace root and selects them by package name.
WCP_PACKAGES := -p ak-ee-wcp-wire -p ak-ee-wcp -p ak-ee-wcp-browser-host -p ak-ee-wcp-e2e
WCP_BUILD_PACKAGES := -p ak-ee-wcp -p ak-ee-wcp-browser-host
WCP_MANIFEST := "${TOP}Cargo.toml"
WCP_TARGET_DIR := "${TOP}cache/wcp"

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${OUT_TARGET}" "cache/wcp"

# `ak_browser.exe` and the DLL are the whole payload: the sign-in window renders
# through the machine's Microsoft Edge WebView2 runtime, so there is no browser
# runtime to fetch, stage or ship.
.PHONY: build
build:
	mkdir -p "${TOP}bin/${OUT_TARGET}"
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
	cp "${TOP}cache/wcp/release/ak_browser.exe" "${TOP}bin/${OUT_TARGET}/"
	cp "${TOP}cache/wcp/release/ak_browser.pdb" "${TOP}bin/${OUT_TARGET}/" || true
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}bin/${OUT_TARGET}/")
endif

# llvm-cov's manual show-env flow rather than `$(call cargo_test,...)`, because
# the e2e tests LoadLibraryW the real ak_cred_provider.dll and spawn the real
# ak_browser.exe, and cargo test/nextest only ever build test harnesses.
# Exporting llvm-cov's env first lets an explicit `cargo build` place both --
# instrumented -- where the test binary resolves them from. AK_WCP_E2E opts in
# to the process-level tests; see e2e/README.md.
#
# One chained shell command on purpose: `.ONESHELL:` is a no-op on the GNU Make
# 3.81 in GnuWin32, so `cd`/`export`/`source` would not carry across lines.
.PHONY: test
test:
	mkdir -p "${TOP}cache"
	cd "${TOP}" && \
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
	cargo clippy \
		--target-dir ${WCP_TARGET_DIR} \
		--manifest-path ${WCP_MANIFEST} \
		${WCP_PACKAGES}
	cargo clippy --fix --allow-dirty \
		--target-dir ${WCP_TARGET_DIR} \
		--manifest-path ${WCP_MANIFEST} \
		${WCP_PACKAGES}
