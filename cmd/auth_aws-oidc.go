package cmd

import (
	"encoding/json"
	"os"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	"github.com/spf13/cobra"
)

var awsOidcCmd = &cobra.Command{
	Use:   "aws-oidc",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		c := sts.New(sts.Options{})
		a, err := c.AssumeRoleWithWebIdentity(cmd.Context(), &sts.AssumeRoleWithWebIdentityInput{
			RoleArn:          aws.String(""),
			RoleSessionName:  aws.String(""),
			WebIdentityToken: aws.String(""),
		})
		if err != nil {
			panic(err)
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
}
