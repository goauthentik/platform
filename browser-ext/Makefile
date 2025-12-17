include ../common.mk

TARGET := browser-ext
FILE_NAME_CHROME := authentik_chrome.zip
FILE_NAME_FIREFOX := authentik_firefox.zip

.PHONY: bump
bump:
	npm version ${VERSION} || true
	yq -i '.version = ${VERSION}' ${PWD}/manifest-chrome.json
	yq -i '.version = ${VERSION}' ${PWD}/manifest-firefox.json
	sed -i 's/VERSION = ".*"/VERSION = ${VERSION}/g' ${PWD}/src/version.ts
	npm run prettier

.PHONY: build
build:
	mkdir -p ${TOP}/bin/${TARGET}
	rm -rf ${PWD}/dist/*
	npm i
	npm run build

.PHONY: package
package: package-chrome package-firefox

.PHONY: package-chrome
package-chrome: build
	rm -f "${TOP}/bin/${TARGET}/${FILE_NAME_CHROME}"
	rm -rf "${TOP}/cache/${TARGET}/chrome"
	mkdir -p "${TOP}/cache/${TARGET}/chrome"
	cp -r "${PWD}/images" "${PWD}/dist" "${PWD}/html" "${TOP}/cache/${TARGET}/chrome"
	cp "${PWD}/manifest-chrome.json" "${TOP}/cache/${TARGET}/chrome/manifest.json"
	cd "${TOP}/cache/${TARGET}/chrome"/ && zip "${TOP}/bin/${TARGET}/${FILE_NAME_CHROME}" -r *

.PHONY: package-firefox
package-firefox: build
	rm -f "${TOP}/bin/${TARGET}/${FILE_NAME_FIREFOX}"
	rm -rf "${TOP}/cache/${TARGET}/firefox"
	mkdir -p "${TOP}/cache/${TARGET}/firefox"
	cp -r "${PWD}/images" "${PWD}/dist" "${PWD}/html" "${TOP}/cache/${TARGET}/firefox"
	cp "${PWD}/manifest-firefox.json" "${TOP}/cache/${TARGET}/firefox/manifest.json"
	cd "${TOP}/cache/${TARGET}/firefox"/ && zip "${TOP}/bin/${TARGET}/${FILE_NAME_FIREFOX}" -r *

.PHONY: lint
lint:
	npm i
	npm run lint-check
	npm run prettier-check
