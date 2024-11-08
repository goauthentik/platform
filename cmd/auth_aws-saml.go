package cmd

import (
	"encoding/json"
	"os"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/sts"
	"github.com/spf13/cobra"
)

type AWSCredentialOutput struct {
	Version         int
	AccessKeyId     string
	SecretAccessKey string
	SessionToken    string
	Expiration      time.Time
}

var awsSamlCmd = &cobra.Command{
	Use:   "aws-saml",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		c := sts.New(sts.Options{})
		a, err := c.AssumeRoleWithSAML(cmd.Context(), &sts.AssumeRoleWithSAMLInput{
			SAMLAssertion: aws.String(""),
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
	authCmd.AddCommand(awsSamlCmd)
}
