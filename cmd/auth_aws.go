/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
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

// awsCmd represents the aws command
var awsCmd = &cobra.Command{
	Use:   "aws",
	Short: "A brief description of your command",
	Long: `A longer description that spans multiple lines and likely contains examples
and usage of using your command. For example:

Cobra is a CLI library for Go that empowers applications.
This application is a tool to generate the needed files
to quickly create a Cobra application.`,
	Run: func(cmd *cobra.Command, args []string) {
		c := sts.Client{}
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
	authCmd.AddCommand(awsCmd)
}
