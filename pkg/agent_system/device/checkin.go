package device

import (
	"context"
	"math/rand/v2"
	"time"

	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/facts"
)

func (ds *Server) checkIn(ctx context.Context, dom *config.DomainConfig) {
	time.Sleep(time.Duration(rand.IntN(30)) * time.Second)
	ds.log.Debug("Starting facts gathering...")
	api, err := dom.APIClient()
	if err != nil {
		ds.log.WithError(err).Warning("failed to get domain API Client")
		return
	}
	req, err := facts.Gather(ds.log)
	if err != nil {
		ds.log.WithError(err).Warning("failed to gather device info")
		return
	}
	ds.log.Debug("Finished facts gathering")
	hr, err := api.EndpointsApi.EndpointsAgentsConnectorsCheckInCreate(ctx).DeviceFactsRequest(*req).Execute()
	if err != nil {
		ds.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to checkin")
	}
}
