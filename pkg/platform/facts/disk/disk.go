package disk

import "goauthentik.io/api/v3"

// Gather collects disk information for the current platform
func Gather() ([]api.DiskRequest, error) {
	return gather()
}
