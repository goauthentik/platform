package user

import "goauthentik.io/api/v3"

func Gather() ([]api.DeviceUserRequest, error) {
	return gather()
}
