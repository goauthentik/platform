include ../../common.mk

TARGET := vpkg/linux
DEB_BUILD_GNU_TYPE := $(shell dpkg-architecture -q DEB_BUILD_GNU_TYPE)

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
	$(call nfpm_package,${TOP}/cmd/cli/package/linux/nfpm.yaml)
	$(call nfpm_package,${TOP}/cmd/agent_system/package/linux/nfpm.yaml)
	$(call nfpm_package,${TOP}/cmd/agent_local/package/linux/nfpm.yaml)
	$(call nfpm_package,${TOP}/nss/nfpm.yaml)
	$(call nfpm_package,${TOP}/pam/nfpm.yaml)
