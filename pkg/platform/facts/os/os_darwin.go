//go:build darwin

package os

import (
	"os/exec"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	version := getMacOSVersion()
	name := getMacOSName()

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  api.DEVICEFACTSOSFAMILY_MAC_OS,
		Name:    new(name),
		Version: new(version),
	}, nil
}

func getMacOSVersion() string {
	cmd := exec.Command("sw_vers", "-productVersion")
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	return strings.TrimSpace(string(output))
}

func getMacOSName() string {
	cmd := exec.Command("sw_vers", "-productName")
	output, err := cmd.Output()
	if err != nil {
		return "macOS"
	}

	return strings.TrimSpace(string(output))
}
