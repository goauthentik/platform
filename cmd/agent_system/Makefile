include ../../common.mk

TARGET := agent_system
BINARY_NAME := ak-sys

build:
	mkdir -p ${TOP}/bin/${TARGET}
	go build \
		-ldflags "${LD_FLAGS}" \
		-v -a -o ${TOP}/bin/${TARGET}/${BINARY_NAME} \
		${TOP}/cmd/${TARGET}

.PHONY: package
package: build
	VERSION=${VERSION} ARCH=${ARCH} \
		go tool github.com/goreleaser/nfpm/v2/cmd/nfpm \
			package \
			-p deb \
			-t ${TOP}/bin/${TARGET} \
			-f ${TOP}/cmd/${TARGET}/package/nfpm.yaml

test-deploy: package
	$(TME) dpkg -i /workspaces/bin/${TARGET}/authentik-sys-agent_${VERSION}_${ARCH}.deb
	$(TME) cp /etc/authentik/host.yaml.dist /etc/authentik/host.yaml
	$(TME) yq -yi '.token = "this-token-is-for-testing-dont-use"' /etc/authentik/host.yaml
	$(TME) systemctl restart ak-sys
