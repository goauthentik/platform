package aws

import (
	"context"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/pb"
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

func GetCredentials(c *client.Client, ctx context.Context, opts CredentialsOpts) *AWSCredentialOutput {
	log := log.WithField("logger", "auth.aws")

	cc := client.NewCache[AWSCredentialOutput](c, &pb.RequestHeader{
		Profile: opts.Profile,
	}, "auth-aws-cache", opts.RoleARN)
	if v, err := cc.Get(ctx); err == nil {
		log.Debug("Got AWS Credentials from cache")
		return &v
	}

	stsc := sts.New(sts.Options{
		Region: opts.Region,
	})
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

	curr, err := c.GetCurrentToken(ctx, &pb.CurrentTokenRequest{
		Header: &pb.RequestHeader{
			Profile: opts.Profile,
		},
		Type: pb.CurrentTokenRequest_VERIFIED,
	})
	if err != nil {
		log.WithError(err).Fatal("failed to get current token")
		return nil
	}

	log.Debug("Fetching AWS Credentials...")
	a, err := stsc.AssumeRoleWithWebIdentity(ctx, &sts.AssumeRoleWithWebIdentityInput{
		RoleArn:          aws.String(opts.RoleARN),
		RoleSessionName:  aws.String(curr.Token.PreferredUsername),
		WebIdentityToken: aws.String(res.AccessToken),
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
	err = cc.Set(ctx, output)
	if err != nil {
		log.WithError(err).Warning("failed to cache AWS Credentials")
	}
	return &output
}
