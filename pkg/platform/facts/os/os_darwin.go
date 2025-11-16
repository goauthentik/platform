//go:build darwin

package os

import (
	"os/exec"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestOs, error) {
	version := getMacOSVersion()
	name := getMacOSName()

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  "darwin",
		Name:    api.PtrString(name),
		Version: api.PtrString(version),
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
