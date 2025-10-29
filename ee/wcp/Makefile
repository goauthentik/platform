include ../../common.mk

.ONESHELL:

TARGET := wcp

.PHONY: clean
clean:
	rm -rf build

.PHONY: build
build:
	mkdir -p "${TOP}/bin/${TARGET}"
	cd "${TOP}/bin/${TARGET}"
	PERL5LIB=/c/Strawberry/perl/vendor/lib cmake -G "Visual Studio 17" "$(CURDIR)"
	cmake --build . --config Release
