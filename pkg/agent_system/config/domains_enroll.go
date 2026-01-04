package config

import (
	"context"
	"fmt"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
	"goauthentik.io/platform/pkg/platform/facts/network"
)

func (dc *DomainConfig) Enroll() error {
	dlog := dc.r.log.WithField("domain", dc.Domain)
	dlog.Info("Enrolling...")
	a, err := dc.APIClient()
	if err != nil {
		return err
	}
	a.GetConfig().AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", dc.Token))
	ctx := common.New(dlog, context.Background())
	hw, err := hardware.Gather(ctx)
	if err != nil {
		return err
	}
	net, err := network.Gather(ctx)
	if err != nil {
		return err
	}
	res, hr, err := a.EndpointsApi.EndpointsAgentsConnectorsEnrollCreate(context.Background()).EnrollRequest(api.EnrollRequest{
		DeviceSerial: hw.Serial,
		DeviceName:   net.Hostname,
	}).Execute()
	if err != nil {
		return ak.HTTPToError(hr, err)
	}
	dc.Token = res.Token
	return nil
}
