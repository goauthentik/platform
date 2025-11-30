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
		Family:  api.DEVICEFACTSOSFAMILY_WINDOWS,
		Name:    name,
		Version: version,
	}, nil
}

func getWindowsProductName() *string {
	cmd := exec.Command("powershell", "-Command",
		`(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').ProductName`)
	output, err := cmd.Output()
	if err != nil {
		return nil
	}
	return api.PtrString(strings.TrimSpace(string(output)))
}

func getWindowsVersion() *string {
	// Try PowerShell for version info
	cmd := exec.Command("powershell", "-Command",
		`(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').ReleaseId`)
	output, err := cmd.Output()
	if err == nil && strings.TrimSpace(string(output)) != "" {
		releaseId := strings.TrimSpace(string(output))

		// Get build number
		cmd = exec.Command("powershell", "-Command",
			`(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').CurrentBuild`)
		buildOutput, buildErr := cmd.Output()
		if buildErr == nil {
			build := strings.TrimSpace(string(buildOutput))
			return api.PtrString(releaseId + " (Build " + build + ")")
		}

		return api.PtrString(releaseId)
	}
	return nil
}
