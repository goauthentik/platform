include ../../common.mk

TARGET := vpkg/linux
DEB_BUILD_GNU_TYPE := $(shell dpkg-architecture -q DEB_BUILD_GNU_TYPE)
export DEB_BUILD_GNU_TYPE

.PHONY: package
package:
	mkdir -p "${TOP}/bin/${TARGET}"
	mkdir -p "${TOP}/bin/cli/completions"
	mkdir -p "${TOP}/bin/agent_system/completions"
	"${TOP}/hack/completions.sh" \
		"${TOP}/cmd/cli" \
		"${TOP}/bin/cli/completions" \
		"cli"
	"${TOP}/hack/completions.sh" \
		"${TOP}/cmd/agent_system" \
		"${TOP}/bin/agent_system/completions" \
		"agent_system"
	$(call nfpm_package,${TOP}/vpkg/linux/cli/nfpm.yaml,cli)
	$(call nfpm_package,${TOP}/vpkg/linux/sysd/nfpm.yaml,agent_system)
	$(call nfpm_package,${TOP}/vpkg/linux/agent/nfpm.yaml,agent_local)
	$(call nfpm_package,${TOP}/vpkg/linux/nss/nfpm.yaml,nss)
	$(call nfpm_package,${TOP}/vpkg/linux/pam/nfpm.yaml,pam)

.PHONY: lint
lint:
	$(call lint_shellcheck,"${PWD}/sysd")
	$(call lint_shellcheck,"${PWD}/agent")
	$(call lint_shellcheck,"${PWD}/nss")
	$(call lint_shellcheck,"${PWD}/pam")
