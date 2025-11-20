package hardware

import "goauthentik.io/api/v3"

// Gather collects hardware information for the current platform
func Gather() (api.DeviceFactsRequestHardware, error) {
	return gather()
}
