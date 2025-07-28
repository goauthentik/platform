package cli

import (
	"encoding/json"
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/cli/auth/aws"
	"goauthentik.io/cli/pkg/cli/client"
)

var awsCmd = &cobra.Command{
	Use:   "aws",
	Short: "Authenticate to AWS with the authentik profile.",
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))
		roleArn := mustFlag(cmd.Flags().GetString("role-arn"))
		region := mustFlag(cmd.Flags().GetString("region"))

		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		cc := aws.GetCredentials(c, cmd.Context(), aws.CredentialsOpts{
			Profile:  profile,
			ClientID: clientId,
			RoleARN:  roleArn,
			Region:   region,
		})
		return json.NewEncoder(os.Stdout).Encode(cc)
	},
}

func init() {
	authCmd.AddCommand(awsCmd)
	awsCmd.Flags().StringP("client-id", "c", "", "Client ID")
	awsCmd.Flags().StringP("role-arn", "r", "", "Role ARN")
	awsCmd.Flags().StringP("region", "e", "eu-central-1", "Region")
	_ = awsCmd.MarkFlagRequired("client-id")
	_ = awsCmd.MarkFlagRequired("role-arn")
}
