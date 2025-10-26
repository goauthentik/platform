include ../../common.mk

TARGET := wcp

.PHONY: clean
clean:


.PHONY: bump
bump:


.PHONY: gen
gen:
	protoc \
		--grpc_out="${PWD}/_gen" \
		--cpp_out="${PWD}/_gen" \
		--plugin=protoc-gen-grpc=$(shell which grpc_cpp_plugin) \
		-I $(PROTO_DIR) \
		$(PROTO_DIR)/**

.PHONY: build
build:
	mkdir -p "${PWD}/build"
	cd "${PWD}/build"
	cmake -G "Visual Studio 17" -A x64 "${PWD}"
	cmake --build "${PWD}/build" --config Release
	gc cef.sln
	devenv cef.sln /build Release
