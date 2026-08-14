include ../../common.mk

.ONESHELL:

OUT_TARGET := wcp
CEF_PIN := $(shell grep -A1 '^name = "cef-dll-sys"' Cargo.lock | grep version | sed -E 's/.*"([0-9.]+)\+.*/\1/')
CEF_PATH := $(TOP)cache/cef

.PHONY: clean
clean:
	cd "${TOP}" && rm -rf "bin/${OUT_TARGET}" "cache/wcp-rs"

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
			--target-dir "${TOP}cache/wcp-rs" \
			--manifest-path "${PWD}/Cargo.toml"
	cp "${TOP}cache/wcp-rs/release/ak_cred_provider.dll" "${TOP}bin/${OUT_TARGET}/"
	cp "${TOP}cache/wcp-rs/release/ak_cred_provider.pdb" "${TOP}bin/${OUT_TARGET}/" || true
	cp "${TOP}cache/wcp-rs/release/ak_cred_provider.dll.lib" "${TOP}bin/${OUT_TARGET}/ak_cred_provider.lib"
	cp "${TOP}cache/wcp-rs/release/ak_cef.exe" "${TOP}bin/${OUT_TARGET}/"
	cp "${TOP}cache/wcp-rs/release/ak_cef.pdb" "${TOP}bin/${OUT_TARGET}/" || true
	cp "${CEF_PATH}"/*.dll "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}"/*.pak "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/icudtl.dat" "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/v8_context_snapshot.bin" "${TOP}bin/${OUT_TARGET}/"
	cp "${CEF_PATH}/vk_swiftshader_icd.json" "${TOP}bin/${OUT_TARGET}/"
	cp -r "${CEF_PATH}/locales" "${TOP}bin/${OUT_TARGET}/"
ifdef SENTRY_AUTH_TOKEN
	@$(call sentry_upload_symbols,"${TOP}bin/${OUT_TARGET}/")
endif

.PHONY: test
test:
	cargo test --manifest-path "${PWD}/Cargo.toml"

.PHONY: lint
lint:
	cargo fmt --all --manifest-path "${PWD}/Cargo.toml"
	cargo clippy --workspace --manifest-path "${PWD}/Cargo.toml"
	cargo clippy --fix --allow-dirty --workspace --manifest-path "${PWD}/Cargo.toml"
