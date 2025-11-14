package device

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/device/serial"
)

func (ds *Server) gather() (*api.DeviceFactsRequest, error) {
	req := &api.DeviceFactsRequest{}
	serial, err := serial.Read()
	if err != nil {
		return nil, err
	}
	req.Hardware = *api.NewNullableDeviceFactsRequestHardware(&api.DeviceFactsRequestHardware{
		Serial: serial,
	})
	return req, nil
}

func (ds *Server) checkIn() {
	req, err := ds.gather()
	if err != nil {
		ds.log.WithError(err).Warning("failed to gather device info")
		return
	}
	_, _, err = ds.api.EndpointsApi.EndpointsAgentsConnectorsCheckInCreate(context.Background()).DeviceFactsRequest(*req).Execute()
	if err != nil {
		ds.log.WithError(err).Warning("failed to checkin")
	}
}
