package cli

import (
	"encoding/json"
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/aws"
)

var awsOidcCmd = &cobra.Command{
	Use:   "aws-oidc",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))
		roleArn := mustFlag(cmd.Flags().GetString("role-arn"))
		region := mustFlag(cmd.Flags().GetString("region"))

		cc := aws.GetCredentials(cmd.Context(), aws.CredentialsOpts{
			Profile:  profile,
			ClientID: clientId,
			RoleARN:  roleArn,
			Region:   region,
		})
		err := json.NewEncoder(os.Stdout).Encode(cc)
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
