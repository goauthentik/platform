//go:build windows

package os

import (
	"os/exec"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestOs, error) {
	name := getWindowsProductName()
	version := getWindowsVersion()

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  "windows",
		Name:    api.PtrString(name),
		Version: api.PtrString(version),
	}, nil
}

func getWindowsProductName() string {
	// Try PowerShell first for better results
	cmd := exec.Command("powershell", "-Command",
		"(Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion').ProductName")
	output, err := cmd.Output()
	if err == nil {
		return strings.TrimSpace(string(output))
	}

	// Fallback to wmic
	return getWMICValue("os", "Caption")
}

func getWindowsVersion() string {
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
			return releaseId + " (Build " + build + ")"
		}

		return releaseId
	}

	// Fallback to wmic
	version := getWMICValue("os", "Version")
	if version == "" {
		version = getWMICValue("os", "BuildNumber")
	}

	return version
}

func getWMICValue(class, property string) string {
	cmd := exec.Command("wmic", class, "get", property, "/value")
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, property+"=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				return strings.TrimSpace(parts[1])
			}
		}
	}

	return ""
}
