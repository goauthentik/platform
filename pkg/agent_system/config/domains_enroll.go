package config

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
	"goauthentik.io/platform/pkg/platform/facts/network"
)

func (dc *DomainConfig) Enroll() error {
	a, err := dc.APIClient()
	if err != nil {
		return err
	}
	hw, err := hardware.Gather()
	if err != nil {
		return err
	}
	net, err := network.Gather()
	if err != nil {
		return err
	}
	res, _, err := a.EndpointsApi.EndpointsAgentsConnectorsEnrollCreate(context.Background()).EnrollRequest(api.EnrollRequest{
		DeviceSerial: hw.Serial,
		DeviceName:   net.Hostname,
	}).Execute()
	if err != nil {
		return err
	}
	dc.Token = res.Token
	return nil
}
