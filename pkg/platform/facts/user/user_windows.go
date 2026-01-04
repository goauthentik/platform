//go:build windows

package user

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() ([]api.DeviceUserRequest, error) {
	var users []api.DeviceUserRequest

	users = getUsersFromPowerShell()
	return users, nil
}

type rawUser struct {
	Name string `json:"Name"`
	SID  struct {
		BinaryLength     int    `json:"BinaryLength"`
		AccountDomainSid string `json:"AccountDomainSid"`
		Value            string `json:"Value"`
	} `json:"SID"`
	FullName string `json:"FullName"`
	Enabled  bool   `json:"Enabled"`
}

func getUsersFromPowerShell() []api.DeviceUserRequest {
	var users []api.DeviceUserRequest

	rusers, err := common.ExecJSON[[]rawUser]("powershell", "-Command", `Get-LocalUser | Select-Object Name,SID,FullName,Enabled | ConvertTo-Json`)
	if err != nil {
		return users
	}

	for _, rawUser := range rusers {
		devUser := api.DeviceUserRequest{
			Id:       rawUser.SID.Value,
			Username: &rawUser.Name,
		}
		if rawUser.FullName != "" {
			devUser.Name = &rawUser.FullName
		}
		users = append(users, devUser)
	}
	return users
}
