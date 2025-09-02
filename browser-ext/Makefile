include ../common.mk

TARGET := browser-ext
BINARY_NAME := ak

build:
	mkdir -p ${TOP}/bin/${TARGET}
	npm i
	npm run build

package: build
	rm -f ${TOP}/browser-ext.pem
	'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' --pack-extension=${PWD}
	mv ${TOP}/browser-ext.crx ${TOP}/bin/${TARGET}
	mv ${TOP}/browser-ext.pem ${TOP}/bin/${TARGET}
