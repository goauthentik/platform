package cli

import (
	"encoding/json"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/aws"
)

var awsCmd = &cobra.Command{
	Use:   "aws",
	Short: "Authenticate to AWS with the authentik profile.",
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
			log.WithError(err).Warning("failed to write AWS credentials")
			os.Exit(1)
		}
	},
}

func init() {
	authCmd.AddCommand(awsCmd)
	awsCmd.Flags().StringP("client-id", "c", "", "Client ID")
	awsCmd.Flags().StringP("role-arn", "r", "", "Role ARN")
	awsCmd.Flags().StringP("region", "e", "eu-central-1", "Region")
	awsCmd.MarkFlagRequired("client-id")
	awsCmd.MarkFlagRequired("role-arn")
}
