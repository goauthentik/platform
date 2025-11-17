package device

import (
	"context"

	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/facts"
)

func (ds *Server) checkIn() {
	ds.log.Debug("Starting facts gathering...")
	req, err := facts.Gather(ds.log)
	if err != nil {
		ds.log.WithError(err).Warning("failed to gather device info")
		return
	}
	ds.log.Debug("Finished facts gathering")
	hr, err := ds.api.EndpointsApi.EndpointsAgentsConnectorsCheckInCreate(context.Background()).DeviceFactsRequest(*req).Execute()
	if err != nil {
		ds.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to checkin")
	}
}
