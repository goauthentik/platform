//go:build darwin

package group

import (
	"os/exec"
	"strings"

	"github.com/micromdm/plist"
	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceGroupRequest, error) {
	var groups []api.DeviceGroupRequest

	cmd := exec.Command("dscl", ".", "list", "/Groups")
	output, err := cmd.Output()
	if err != nil {
		return groups, err
	}

	groupNames := strings.Split(strings.TrimSpace(string(output)), "\n")

	for _, groupName := range groupNames {
		groupName = strings.TrimSpace(groupName)
		if groupName == "" {
			continue
		}

		// Skip system groups starting with underscore (optional)
		if strings.HasPrefix(groupName, "_") {
			continue
		}

		groupInfo := getGroupInfoFromDscl(groupName)
		if groupInfo.Id != "" {
			groups = append(groups, groupInfo)
		}
	}

	return groups, nil
}

type dscGroupInfo struct {
	PrimaryGroupID []string `plist:"dsAttrTypeStandard:PrimaryGroupID"`
}

func getGroupInfoFromDscl(groupName string) api.DeviceGroupRequest {
	groupInfo := api.DeviceGroupRequest{Name: api.PtrString(groupName)}

	cmd := exec.Command("dscl", "-plist", ".", "read", "/Groups/"+groupName, "PrimaryGroupID")

	output, err := cmd.Output()
	if err != nil {
		return groupInfo
	}
	dp := dscGroupInfo{}
	err = plist.Unmarshal(output, &dp)
	if err != nil {
		return groupInfo
	}
	groupInfo.Id = dp.PrimaryGroupID[0]
	return groupInfo
}
