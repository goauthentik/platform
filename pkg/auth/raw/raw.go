package raw

import (
	"context"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
)

type CredentialsOpts struct {
	Profile  string
	ClientID string
}

type RawCredentialOutput struct {
	AccessToken string
}

func GetCredentials(ctx context.Context, opts CredentialsOpts) *RawCredentialOutput {
	log := log.WithField("logger", "auth.raw")
	mgr := storage.Manager()
	prof := mgr.Get().Profiles[opts.Profile]

	nt, err := token.CachedExchangeToken(opts.Profile, prof, token.DefaultExchangeOpts(opts.ClientID))
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	output := RawCredentialOutput{
		AccessToken: nt.RawAccessToken,
	}
	return &output
}
