package os

import "goauthentik.io/api/v3"

// Gather collects OS information for the current platform
func Gather() (api.DeviceFactsRequestOs, error) {
	return gather()
}
