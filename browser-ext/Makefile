include ../common.mk

TARGET := browser-ext
FILE_NAME := authentik.zip

.PHONY: bump
bump:
	npm version ${VERSION} || true
	yq -i '.version = ${VERSION}' manifest.json

build:
	mkdir -p ${TOP}/bin/${TARGET}
	npm i
	npm run prettier
	npm run build

package: build
	zip ${TOP}/bin/${TARGET}/${FILE_NAME} -r manifest.json images dist

lint:
	npm i
	npm run lint-check
	npm run prettier-check
