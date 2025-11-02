include ../common.mk

PROTO_OUT := "${PWD}/src/generated"
TARGET = utils_rs

.PHONY: gen
gen:
	cargo install protoc-gen-prost
	cargo install protoc-gen-tonic
	mkdir -p $(PROTO_OUT)
	protoc \
		--prost_out=$(PROTO_OUT) \
		--tonic_out=$(PROTO_OUT) \
		--tonic_opt=no_server \
		-I $(PROTO_DIR) \
		${PROTO_DIR}/*
	cargo fmt

.PHONY: lint
lint:
	cargo clippy --fix --allow-dirty

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	mkdir -p ${TOP}/cache/${TARGET}
	cargo build \
		--target-dir ${TOP}/cache/${TARGET} \
		--verbose \
		--release
