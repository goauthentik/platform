package config

import (
	"context"
	"os"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/device/serial"
)

func (dc *DomainConfig) Enroll() error {
	a, err := dc.APIClient()
	if err != nil {
		return err
	}
	serial, err := serial.Read()
	if err != nil {
		return err
	}
	hostname, err := os.Hostname()
	if err != nil {
		return err
	}
	res, _, err := a.EndpointsApi.EndpointsAgentsConnectorsEnrollCreate(context.Background()).EnrollRequest(api.EnrollRequest{
		DeviceSerial: serial,
		DeviceName:   hostname,
	}).Execute()
	if err != nil {
		return err
	}
	dc.Token = res.Token
	return nil
}
