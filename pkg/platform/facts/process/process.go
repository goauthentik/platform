package process

import "goauthentik.io/api/v3"

// Gather collects process information for the current platform
func Gather() ([]api.ProcessRequest, error) {
	return gather()
}
