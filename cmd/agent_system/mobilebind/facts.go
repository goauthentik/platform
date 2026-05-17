package mobilebind

import (
	"context"
	"encoding/json"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func Facts() string {
	facts, err := facts.Gather(common.New(logger, context.Background()))
	if err != nil {
		logger.WithError(err).Warning("failed to get all facts")
	}
	s, err := json.Marshal(facts)
	if err != nil {
		logger.WithError(err).Warning("failed to json marshal")
		return ""
	}
	return string(s)
}

func FactsSetSerial(serial string) {
	hardware.StaticHardware = &api.HardwareRequest{
		Serial: serial,
	}
}
