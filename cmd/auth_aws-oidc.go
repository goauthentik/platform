package cmd

import (
	"encoding/json"
	"os"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/cfg"
)

var awsOidcCmd = &cobra.Command{
	Use:   "aws-oidc",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		c := sts.New(sts.Options{
			Region: "eu-central-1",
		})
		mgr, err := cfg.Manager()
		if err != nil {
			log.WithError(err).Panic("failed to initialise config manager")
		}
		profile := mustFlag(cmd.Flags().GetString("profile"))
		prof := mgr.Get().Profiles[profile]

		clientId := mustFlag(cmd.Flags().GetString("client-id"))
		roleArn := mustFlag(cmd.Flags().GetString("role-arn"))

		nt, err := ak.ExchangeToken(prof, ak.ExchangeOpts{
			ClientID: clientId,
		})
		if err != nil {
			log.WithError(err).Fatal("failed to exchange token")
		}

		a, err := c.AssumeRoleWithWebIdentity(cmd.Context(), &sts.AssumeRoleWithWebIdentityInput{
			RoleArn:          aws.String(roleArn),
			RoleSessionName:  aws.String("temp"),
			WebIdentityToken: aws.String(nt.AccessToken),
		})
		if err != nil {
			log.WithError(err).Panic("failed to assume WebIdentity")
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
}
