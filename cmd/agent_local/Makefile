include ../../common.mk

TARGET := agent_local
ifeq ($(OS),Windows_NT)
BINARY_NAME := ak-agent.exe
else
BINARY_NAME := ak-agent
endif

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

test-deploy: build
	sudo dpkg -i ${TOP}/bin/${TARGET}/authentik-agent_${VERSION}_${ARCH}.deb
