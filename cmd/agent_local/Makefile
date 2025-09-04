include ../../common.mk

TARGET := agent_local
BINARY_NAME := ak-agent

build:
	mkdir -p ${TOP}/bin/${TARGET}
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o ${TOP}/bin/${TARGET}/${BINARY_NAME} \
		${TOP}/cmd/${TARGET}

.PHONY: package
package: build package-macos package-deb

package-macos: build
	$(MAKE) -C ../cli build
	cp -R "${PWD}/package/macos/authentik Agent.app" ${TOP}/bin/${TARGET}/
	mkdir -p "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS"
	cp ${TOP}/bin/${TARGET}/${BINARY_NAME} "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS/"
	cp ${TOP}/bin/cli/ak "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS/"
	ln -s ak "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS/ak-vault" || true
	ln -s ak "${TOP}/bin/${TARGET}/authentik Agent.app/Contents/MacOS/ak-browser-support" || true
# 	codesign --options runtime --deep --entitlements "package/entitlements.plist" --timestamp --sign "AEBEDA5994467C45F0AED1ACAE74DE316C8E3491" "${TOP}/bin/${TARGET}/authentik Agent.app"

package-deb: build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
