//go:build windows

package group

import (
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceGroupRequest, error) {
	var groups []api.DeviceGroupRequest

	cmd := exec.Command("powershell", "-Command",
		"Get-LocalGroup | ForEach-Object { \"$($_.Name)|$($_.SID)\" }")
	output, err := cmd.Output()
	if err != nil {
		return groups, err
	}

	lines := strings.Split(strings.TrimSpace(string(output)), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		parts := strings.Split(line, "|")
		if len(parts) >= 2 {
			groupName := strings.TrimSpace(parts[0])
			sid := strings.TrimSpace(parts[1])

			groupInfo := api.DeviceGroupRequest{
				Id:   sid,
				Name: api.PtrString(groupName),
			}

			groups = append(groups, groupInfo)
		}
	}

	return groups, nil
}
