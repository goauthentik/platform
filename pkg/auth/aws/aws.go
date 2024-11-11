package aws

import (
	"context"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

type CredentialsOpts struct {
	Profile  string
	ClientID string
	// AWS specific things
	RoleARN string
	Region  string
}

type AWSCredentialOutput struct {
	Version         int
	AccessKeyId     string
	SecretAccessKey string
	SessionToken    string
	Expiration      time.Time
}

func (aco AWSCredentialOutput) Expiry() time.Time {
	return aco.Expiration
}

func GetCredentials(ctx context.Context, opts CredentialsOpts) *AWSCredentialOutput {
	log := log.WithField("logger", "auth.aws")
	mgr := storage.Manager()
	prof := mgr.Get().Profiles[opts.Profile]

	c := sts.New(sts.Options{
		Region: opts.Region,
	})
	nt, err := ak.CachedExchangeToken(opts.Profile, prof, ak.ExchangeOpts{
		ClientID: opts.ClientID,
	})
	if err != nil {
		log.WithError(err).Fatal("failed to exchange token")
		return nil
	}

	cc := storage.NewCache[AWSCredentialOutput]("auth-aws-cache", opts.Profile, opts.RoleARN)
	if v, err := cc.Get(); err == nil {
		log.Debug("Got AWS Credentials from cache")
		return &v
	}

	log.Debug("Fetching AWS Credentials...")
	a, err := c.AssumeRoleWithWebIdentity(ctx, &sts.AssumeRoleWithWebIdentityInput{
		RoleArn:          aws.String(opts.RoleARN),
		RoleSessionName:  aws.String("temp"),
		WebIdentityToken: aws.String(nt.AccessToken),
	})
	if err != nil {
		log.WithError(err).Panic("failed to assume WebIdentity")
		return nil
	}
	output := AWSCredentialOutput{
		Version:         1,
		AccessKeyId:     *a.Credentials.AccessKeyId,
		SecretAccessKey: *a.Credentials.SecretAccessKey,
		SessionToken:    *a.Credentials.SessionToken,
		Expiration:      *a.Credentials.Expiration,
	}
	err = cc.Set(output)
	if err != nil {
		log.WithError(err).Warning("failed to cache AWS Credentials")
	}
	return &output
}
