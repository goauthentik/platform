include ../../common.mk

TARGET := agent_local
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-agent.exe
else
BINARY_NAME := ak-agent
endif

MACOS_APP_NAME := authentik Agent.app
MACOS_SIGN_CERT := 33B1F66E06FD7D8088F2301D240AE13F00399685

lint:
	$(call lint_shellcheck,"${TOP}/cmd/${TARGET}/package/linux/_deb")
	$(call lint_shellcheck,"${TOP}/cmd/${TARGET}/package/linux/_rpm")
	$(call lint_shellcheck,"${TOP}/cmd/${TARGET}/package/macos/scripts")

clean:
	cd "${TOP}" && rm -rf "bin/${TARGET}"

build:
	mkdir -p "${TOP}/bin/${TARGET}"
ifeq ($(OS),Windows_NT)
	$(call go_generate_resources,authentik Platform Agent)
	# build with -H=windowsgui to run as a true systray application
	$(eval LD_FLAGS := -H=windowsgui)
endif
	go build \
		${GO_BUILD_FLAGS} \
		-o "${TOP}/bin/${TARGET}/${BINARY_NAME}" \
		"${TOP}/cmd/${TARGET}"

.PHONY: bump
bump:
	"${TOP}/hack/plist.py" "${PWD}/package/macos/${MACOS_APP_NAME}/Contents/Info.plist" "CFBundleVersion" "${VERSION}"
	"${TOP}/hack/plist.py" "${PWD}/package/macos/${MACOS_APP_NAME}/Contents/Info.plist" "CFBundleShortVersionString" "${VERSION}"

.PHONY: package-gnu/linux
package-gnu/linux: clean build
	$(call nfpm_package,${TOP}/cmd/${TARGET}/package/linux/nfpm.yaml)

test-deploy: package
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
