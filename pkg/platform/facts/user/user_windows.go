//go:build windows

package user

import (
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceUserRequest, error) {
	var users []api.DeviceUserRequest

	// Try PowerShell first for better results
	users = getUsersFromPowerShell()
	if len(users) > 0 {
		return users, nil
	}

	// Fallback to wmic
	return getUsersFromWMIC()
}

func getUsersFromPowerShell() []api.DeviceUserRequest {
	var users []api.DeviceUserRequest

	cmd := exec.Command("powershell", "-Command",
		"Get-LocalUser | Select-Object Name,SID,FullName,Enabled | ConvertTo-Json")
	output, err := cmd.Output()
	if err != nil {
		return users
	}

	// Parse JSON output would be ideal, but for simplicity, let's parse text
	cmd = exec.Command("powershell", "-Command",
		"Get-LocalUser | ForEach-Object { \"$($_.Name)|$($_.SID)|$($_.FullName)|$($_.Enabled)\" }")
	output, err = cmd.Output()
	if err != nil {
		return users
	}

	lines := strings.Split(strings.TrimSpace(string(output)), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		parts := strings.Split(line, "|")
		if len(parts) >= 4 {
			username := strings.TrimSpace(parts[0])
			sid := strings.TrimSpace(parts[1])
			fullName := strings.TrimSpace(parts[2])

			userInfo := api.DeviceUserRequest{
				Id:       sid,
				Username: api.PtrString(username),
			}
			if fullName != "" {
				userInfo.Name = api.PtrString(fullName)
			}

			users = append(users, userInfo)
		}
	}

	return users
}

func getUsersFromWMIC() ([]api.DeviceUserRequest, error) {
	var users []api.DeviceUserRequest

	cmd := exec.Command("wmic", "useraccount", "get", "Name,SID,FullName,Disabled", "/format:csv")
	output, err := cmd.Output()
	if err != nil {
		return users, err
	}

	lines := strings.Split(string(output), "\n")
	for i, line := range lines {
		if i == 0 || strings.TrimSpace(line) == "" {
			continue // Skip header and empty lines
		}

		parts := strings.Split(line, ",")
		if len(parts) < 5 {
			continue
		}

		fullName := strings.TrimSpace(parts[2])
		username := strings.TrimSpace(parts[3])
		sid := strings.TrimSpace(parts[4])

		userInfo := api.DeviceUserRequest{
			Id:       sid,
			Username: api.PtrString(username),
		}
		if fullName != "" {
			userInfo.Name = api.PtrString(fullName)
		}

		users = append(users, userInfo)
	}

	return users, nil
}
