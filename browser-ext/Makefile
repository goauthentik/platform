include ../common.mk

TARGET := browser-ext
FILE_NAME := authentik.zip

.PHONY: bump
bump:
	npm version ${VERSION} || true
	yq -i '.version = ${VERSION}' ${PWD}/manifest.json
	sed -i 's/VERSION = ".*"/VERSION = ${VERSION}/g' ${PWD}/src/version.ts
	npm run prettier

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	rm -rf ${PWD}/dist/*
	npm i
	npm run build

.PHONY: package
package: build
	rm -f ${TOP}/bin/${TARGET}/${FILE_NAME}
	zip ${TOP}/bin/${TARGET}/${FILE_NAME} -r manifest.json images dist html

.PHONY: lint
lint:
	npm i
	npm run lint-check
	npm run prettier-check
