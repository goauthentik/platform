package device

import (
	"context"
	"math/rand/v2"
	"time"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/facts"
)

func (ds *Server) checkIn(ctx context.Context, dom *config.DomainConfig) error {
	time.Sleep(time.Duration(rand.IntN(30)) * time.Second)
	ds.log.Debug("Starting facts gathering...")
	api, err := dom.APIClient()
	if err != nil {
		return errors.Wrap(err, "failed to get domain API Client")
	}
	req, err := facts.Gather(ds.log)
	if err != nil {
		return errors.Wrap(err, "failed to gather device info")
	}
	ds.log.Debug("Finished facts gathering")
	hr, err := api.EndpointsApi.EndpointsAgentsConnectorsCheckInCreate(ctx).DeviceFactsRequest(*req).Execute()
	if err != nil {
		return errors.Wrap(ak.HTTPToError(hr, err), "failed to checkin")
	}
	return nil
}
