include ../common.mk

TARGET := browser-ext
FILE_NAME := authentik.zip

.PHONY: bump
bump:
	npm version ${VERSION} || true
	yq -i '.version = ${VERSION}' manifest.json
	npm run prettier

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	npm i
	npm run build

.PHONY: package
package: build
	zip ${TOP}/bin/${TARGET}/${FILE_NAME} -r manifest.json images dist

.PHONY: lint
lint:
	npm i
	npm run lint-check
	npm run prettier-check
