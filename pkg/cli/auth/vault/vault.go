package vault

import (
	"context"
	"time"

	"github.com/hashicorp/vault-client-go"
	"github.com/hashicorp/vault-client-go/schema"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
)

type CredentialsOpts struct {
	Profile  string
	ClientID string
	// Vault specific things
	MountPath string
	Role      string
}

type VaultCredentialOutput struct {
	*vault.ResponseAuth
}

func (vco VaultCredentialOutput) Expiry() time.Time {
	return time.Now().Add(time.Duration(vco.LeaseDuration) * time.Second)
}

func GetCredentials(ctx context.Context, opts CredentialsOpts) *VaultCredentialOutput {
	log := log.WithField("logger", "auth.vault")
	mgr := storage.Manager()
	prof := mgr.Get().Profiles[opts.Profile]

	cc := storage.NewCache[VaultCredentialOutput]("auth-vault-cache", opts.Profile, opts.Role)
	if v, err := cc.Get(); err == nil {
		log.Debug("Got Vault Credentials from cache")
		return &v
	}

	client, err := vault.New(
		vault.WithEnvironment(),
	)
	if err != nil {
		log.WithError(err).Fatal("failed to create vault client")
		return nil
	}

	nt, err := token.CachedExchangeToken(opts.Profile, prof, token.DefaultExchangeOpts(opts.ClientID))
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	res, err := client.Auth.JwtLogin(ctx, schema.JwtLoginRequest{
		Jwt:  nt.RawAccessToken,
		Role: opts.Role,
	}, vault.WithMountPath(opts.MountPath))
	if err != nil {
		log.WithError(err).Fatal("failed to authenticate to vault")
		return nil
	}
	output := VaultCredentialOutput{res.Auth}
	err = cc.Set(output)
	if err != nil {
		log.WithError(err).Warning("failed to cache vault Credentials")
	}
	return &output
}
