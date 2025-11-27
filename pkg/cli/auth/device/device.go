package device

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/pb"
)

type CredentialsOpts struct {
	Profile    string
	DeviceName string
}

type DeviceCredentialOutput struct {
	AccessToken string
	Expiration  time.Time
}

func (dco DeviceCredentialOutput) Expiry() time.Time {
	return dco.Expiration
}

func GetCredentials(c *client.AgentClient, ctx context.Context, opts CredentialsOpts) *DeviceCredentialOutput {
	log := log.WithField("logger", "auth.device")

	cc := client.NewCache[DeviceCredentialOutput](c, &pb.RequestHeader{
		Profile: opts.Profile,
	}, "auth-device-cache", opts.DeviceName)
	if v, err := cc.Get(ctx); err == nil {
		log.Debug("Got Device Credentials from cache")
		return &v
	}

	dt, err := c.DeviceTokenExchange(ctx, &pb.DeviceTokenExchangeRequest{
		Header: &pb.RequestHeader{
			Profile: opts.Profile,
		},
		DeviceName: opts.DeviceName,
	})
	if err != nil {
		log.WithError(err).Warning("failed to get device token")
		return nil
	}

	o := DeviceCredentialOutput{
		AccessToken: dt.AccessToken,
		Expiration:  time.Now().Add(time.Duration(dt.ExpiresIn) * time.Second),
	}
	err = cc.Set(ctx, o)
	if err != nil {
		log.WithError(err).Warning("failed to cache device token")
	}
	return &o
}
