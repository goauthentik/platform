//go:build linux

package group

import (
	"bufio"
	"os"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceGroupRequest, error) {
	return getGroupsFromGroupFile()
}

func getGroupsFromGroupFile() ([]api.DeviceGroupRequest, error) {
	var groups []api.DeviceGroupRequest

	file, err := os.Open("/etc/group")
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		parts := strings.Split(line, ":")
		if len(parts) < 4 {
			continue
		}

		groupName := parts[0]
		gid := parts[2]

		groupInfo := api.DeviceGroupRequest{
			Id:   gid,
			Name: api.PtrString(groupName),
		}

		groups = append(groups, groupInfo)
	}

	return groups, scanner.Err()
}
