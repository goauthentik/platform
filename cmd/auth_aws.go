package cmd

import (
	"encoding/json"
	"os"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

type AWSCredentialOutput struct {
	Version         int
	AccessKeyId     string
	SecretAccessKey string
	SessionToken    string
	Expiration      time.Time
}

var awsOidcCmd = &cobra.Command{
	Use:   "aws-oidc",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		mgr := storage.Manager()
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))
		roleArn := mustFlag(cmd.Flags().GetString("role-arn"))
		region := mustFlag(cmd.Flags().GetString("region"))
		prof := mgr.Get().Profiles[profile]

		c := sts.New(sts.Options{
			Region: region,
		})
		nt, err := ak.CachedExchangeToken(profile, prof, ak.ExchangeOpts{
			ClientID: clientId,
		})
		if err != nil {
			log.WithError(err).Fatal("failed to exchange token")
			return
		}

		a, err := c.AssumeRoleWithWebIdentity(cmd.Context(), &sts.AssumeRoleWithWebIdentityInput{
			RoleArn:          aws.String(roleArn),
			RoleSessionName:  aws.String("temp"),
			WebIdentityToken: aws.String(nt.AccessToken),
		})
		if err != nil {
			log.WithError(err).Panic("failed to assume WebIdentity")
			return
		}
		output := AWSCredentialOutput{
			Version:         1,
			AccessKeyId:     *a.Credentials.AccessKeyId,
			SecretAccessKey: *a.Credentials.SecretAccessKey,
			SessionToken:    *a.Credentials.SessionToken,
			Expiration:      *a.Credentials.Expiration,
		}
		err = json.NewEncoder(os.Stdout).Encode(output)
		if err != nil {
			panic(err)
		}
	},
}

func init() {
	authCmd.AddCommand(awsOidcCmd)
	awsOidcCmd.Flags().StringP("client-id", "c", "", "Client ID")
	awsOidcCmd.Flags().StringP("role-arn", "r", "", "Role ARN")
	awsOidcCmd.Flags().StringP("region", "e", "eu-central-1", "Region")
}
