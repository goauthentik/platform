//go:build darwin

package group

import (
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
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

	dp, err := common.ExecPlist[dscGroupInfo]("dscl", "-plist", ".", "read", "/Groups/"+groupName, "PrimaryGroupID")
	if err != nil {
		return groupInfo
	}
	groupInfo.Id = dp.PrimaryGroupID[0]
	return groupInfo
}
