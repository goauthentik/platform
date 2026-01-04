package network

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
)

// Gather collects network information for the current platform
func Gather(log *log.Entry) (*api.DeviceFactsRequestNetwork, error) {
	log.Debug("Gathering...")
	return gather(log)
}
