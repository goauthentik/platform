include ../common.mk

PROTO_OUT := "${PWD}/src/generated"

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
