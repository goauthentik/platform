package k8s

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/cli/client"
	"goauthentik.io/cli/pkg/pb"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientauthenticationv1 "k8s.io/client-go/pkg/apis/clientauthentication/v1"
)

type CredentialsOpts struct {
	Profile  string
	ClientID string
}

type KubeCredentialOutput struct {
	*clientauthenticationv1.ExecCredentialStatus
}

func (kco KubeCredentialOutput) Expiry() time.Time {
	return kco.ExpirationTimestamp.Time
}

func GetCredentials(c *client.Client, ctx context.Context, opts CredentialsOpts) *KubeCredentialOutput {
	log := log.WithField("logger", "auth.kube")

	cc := client.NewCache[KubeCredentialOutput](c, &pb.RequestHeader{
		Profile: opts.Profile,
	}, "auth-kube-cache", opts.Profile, opts.ClientID)
	if v, err := cc.Get(ctx); err == nil {
		log.Debug("Got kube Credentials from cache")
		return &v
	}

	res, err := c.CachedTokenExchange(ctx, &pb.TokenExchangeRequest{
		Header: &pb.RequestHeader{
			Profile: opts.Profile,
		},
		ClientId: opts.ClientID,
	})
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	output := KubeCredentialOutput{
		&clientauthenticationv1.ExecCredentialStatus{
			Token: res.AccessToken,
			ExpirationTimestamp: &v1.Time{
				Time: time.Now().Add(time.Duration(res.ExpiresIn) * time.Second),
			},
		},
	}
	err = cc.Set(ctx, output)
	if err != nil {
		log.WithError(err).Warning("failed to cache kube Credentials")
	}
	return &output
}
