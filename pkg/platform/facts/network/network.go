package network

import "goauthentik.io/api/v3"

// Gather collects network information for the current platform
func Gather() (api.DeviceFactsRequestNetwork, error) {
	return gather()
}
