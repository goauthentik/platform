package raw

import (
	"context"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent/client"
	"goauthentik.io/platform/pkg/pb"
)

type CredentialsOpts struct {
	Profile  string
	ClientID string
}

type RawCredentialOutput struct {
	AccessToken string
}

func GetCredentials(client *client.AgentClient, ctx context.Context, opts CredentialsOpts) *RawCredentialOutput {
	log := log.WithField("logger", "auth.raw")

	res, err := client.CachedTokenExchange(ctx, &pb.TokenExchangeRequest{
		Header: &pb.RequestHeader{
			Profile: opts.Profile,
		},
		ClientId: opts.ClientID,
	})
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	output := RawCredentialOutput{
		AccessToken: res.AccessToken,
	}
	return &output
}
