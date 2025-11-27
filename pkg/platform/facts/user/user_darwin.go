//go:build darwin

package user

import (
	"os/exec"
	"strings"

	"github.com/micromdm/plist"
	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceUserRequest, error) {
	var users []api.DeviceUserRequest

	cmd := exec.Command("dscl", ".", "list", "/Users")
	output, err := cmd.Output()
	if err != nil {
		return users, err
	}

	usernames := strings.Split(strings.TrimSpace(string(output)), "\n")

	for _, username := range usernames {
		username = strings.TrimSpace(username)
		userInfo := getUserInfoFromDscl(username)
		if userInfo.Id != "" {
			users = append(users, userInfo)
		}
	}

	return users, nil
}

type dsclUserInfo struct {
	UniqueID         []string `plist:"dsAttrTypeStandard:UniqueID"`
	RealName         []string `plist:"dsAttrTypeStandard:RealName"`
	NFSHomeDirectory []string `plist:"dsAttrTypeStandard:NFSHomeDirectory"`
}

func getUserInfoFromDscl(username string) api.DeviceUserRequest {
	userInfo := api.DeviceUserRequest{Username: api.PtrString(username)}

	cmd := exec.Command("dscl", "-plist", ".", "read", "/Users/"+username)
	output, err := cmd.Output()
	if err != nil {
		return userInfo
	}
	dp := dsclUserInfo{}
	err = plist.Unmarshal(output, &dp)
	if err != nil {
		return userInfo
	}

	userInfo.Id = dp.UniqueID[0]
	if len(dp.RealName) > 0 {
		userInfo.Name = api.PtrString(dp.RealName[0])
	}
	if len(dp.NFSHomeDirectory) > 0 {
		userInfo.Home = api.PtrString(dp.NFSHomeDirectory[0])
	}
	return userInfo
}
