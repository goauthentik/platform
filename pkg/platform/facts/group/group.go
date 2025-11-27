package group

import "goauthentik.io/api/v3"

func Gather() ([]api.DeviceGroupRequest, error) {
	return gather()
}
