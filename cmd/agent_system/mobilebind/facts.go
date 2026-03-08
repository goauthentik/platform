package mobilebind

import (
	"context"
	"encoding/json"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func Facts() string {
	facts, err := facts.Gather(common.New(log.WithField("cmd", "facts"), context.Background()))
	if err != nil {
		log.WithError(err).Warning("failed to get all facts")
	}
	s, err := json.Marshal(facts)
	if err != nil {
		log.WithError(err).Warning("failed to json marshal")
		return ""
	}
	return string(s)
}

func FactsSetSerial(serial string) {
	hardware.StaticHardware = &api.DeviceFactsRequestHardware{
		Serial: serial,
	}
}
