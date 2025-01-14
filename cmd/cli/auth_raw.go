package cli

import (
	"encoding/json"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/raw"
)

var rawCmd = &cobra.Command{
	Use:   "raw",
	Short: "Authenticate to arbitrary API calls.",
	Run: func(cmd *cobra.Command, args []string) {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		cc := raw.GetCredentials(cmd.Context(), raw.CredentialsOpts{
			Profile:  profile,
			ClientID: clientId,
		})
		err := json.NewEncoder(os.Stdout).Encode(cc)
		if err != nil {
			log.WithError(err).Warning("failed to write raw credentials")
			os.Exit(1)
		}
	},
}

func init() {
	authCmd.AddCommand(rawCmd)
	rawCmd.Flags().StringP("client-id", "c", "", "Client ID")
	_ = rawCmd.MarkFlagRequired("client-id")
}
