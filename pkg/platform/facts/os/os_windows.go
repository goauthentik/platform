//go:build windows

package os

import (
	"os/exec"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() (api.DeviceFactsRequestOs, error) {
	name := getWindowsProductName()
	version := getWindowsVersion()

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  api.DEVICEFACTSOSFAMILY_WINDOWS,
		Name:    name,
		Version: version,
	}, nil
}

func getWindowsProductName() *string {
	// Try PowerShell first for better results
	cmd := exec.Command("powershell", "-Command",
		"(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion').ProductName")
	output, err := cmd.Output()
	if err == nil {
		return api.PtrString(strings.TrimSpace(string(output)))
	}

	// Fallback to wmic
	return common.GetWMICValue("os", "Caption")
}

func getWindowsVersion() *string {
	// Try PowerShell for version info
	cmd := exec.Command("powershell", "-Command",
		"(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion').ReleaseId")
	output, err := cmd.Output()
	if err == nil && strings.TrimSpace(string(output)) != "" {
		releaseId := strings.TrimSpace(string(output))

		// Get build number
		cmd = exec.Command("powershell", "-Command",
			"(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion').CurrentBuild")
		buildOutput, buildErr := cmd.Output()
		if buildErr == nil {
			build := strings.TrimSpace(string(buildOutput))
			return api.PtrString(releaseId + " (Build " + build + ")")
		}

		return api.PtrString(releaseId)
	}

	// Fallback to wmic
	version := common.GetWMICValue("os", "Version")
	if version == nil {
		version = common.GetWMICValue("os", "BuildNumber")
	}

	return version
}
