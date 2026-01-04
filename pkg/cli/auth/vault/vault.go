package vault

import (
	"context"
	"time"

	"github.com/hashicorp/vault-client-go"
	"github.com/hashicorp/vault-client-go/schema"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent/client"
	"goauthentik.io/platform/pkg/pb"
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

func GetCredentials(c *client.AgentClient, ctx context.Context, opts CredentialsOpts) *VaultCredentialOutput {
	log := log.WithField("logger", "auth.vault")

	cc := client.NewCache[VaultCredentialOutput](c, &pb.RequestHeader{
		Profile: opts.Profile,
	}, "auth-vault-cache", opts.Role)
	if v, err := cc.Get(ctx); err == nil {
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

	nt, err := c.CachedTokenExchange(ctx, &pb.TokenExchangeRequest{
		Header: &pb.RequestHeader{
			Profile: opts.Profile,
		},
		ClientId: opts.ClientID,
	})
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	res, err := client.Auth.JwtLogin(ctx, schema.JwtLoginRequest{
		Jwt:  nt.AccessToken,
		Role: opts.Role,
	}, vault.WithMountPath(opts.MountPath))
	if err != nil {
		log.WithError(err).Fatal("failed to authenticate to vault")
		return nil
	}
	output := VaultCredentialOutput{res.Auth}
	err = cc.Set(ctx, output)
	if err != nil {
		log.WithError(err).Warning("failed to cache vault Credentials")
	}
	return &output
}
