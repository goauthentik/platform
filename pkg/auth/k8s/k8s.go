package k8s

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
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
	return kco.ExecCredentialStatus.ExpirationTimestamp.Time
}

func GetCredentials(ctx context.Context, opts CredentialsOpts) *KubeCredentialOutput {
	log := log.WithField("logger", "auth.kube")
	mgr := storage.Manager()
	prof := mgr.Get().Profiles[opts.Profile]

	cc := storage.NewCache[KubeCredentialOutput]("auth-kube-cache", opts.Profile, opts.ClientID)
	if v, err := cc.Get(); err == nil {
		log.Debug("Got kube Credentials from cache")
		return &v
	}

	nt, err := token.CachedExchangeToken(opts.Profile, prof, token.DefaultExchangeOpts(opts.ClientID))
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	output := KubeCredentialOutput{
		&clientauthenticationv1.ExecCredentialStatus{
			Token: nt.RawAccessToken,
			ExpirationTimestamp: &v1.Time{
				Time: time.Now().Add(time.Duration(nt.ExpiresIn) * time.Second),
			},
		},
	}
	err = cc.Set(output)
	if err != nil {
		log.WithError(err).Warning("failed to cache kube Credentials")
	}
	return &output
}
