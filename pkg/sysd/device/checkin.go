package device

import (
	"context"
	"math/rand/v2"
	"time"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/facts"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/sysd/config"
)

func (ds *Server) checkIn(ctx context.Context, dom *config.DomainConfig) error {
	time.Sleep(time.Duration(rand.IntN(30)) * time.Second)
	ds.log.Debug("Starting facts gathering...")
	api, err := dom.APIClient()
	if err != nil {
		return errors.Wrap(err, "failed to get domain API Client")
	}
	fctx := common.New(ds.log, ctx)
	req, err := facts.Gather(fctx)
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
