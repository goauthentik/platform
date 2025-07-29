package cli

import (
	"encoding/json"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/cli/auth/raw"
	"goauthentik.io/cli/pkg/cli/client"
)

var rawCmd = &cobra.Command{
	Use:   "raw",
	Short: "Authenticate to arbitrary API calls.",
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		c, err := client.New(socketPath)
		if err != nil {
			return err
		}

		cc := raw.GetCredentials(c, cmd.Context(), raw.CredentialsOpts{
			Profile:  profile,
			ClientID: clientId,
		})
		err = json.NewEncoder(os.Stdout).Encode(cc)
		if err != nil {
			log.WithError(err).Warning("failed to write raw credentials")
			return err
		}
		return nil
	},
}

func init() {
	authCmd.AddCommand(rawCmd)
	rawCmd.Flags().StringP("client-id", "c", "", "Client ID")
	_ = rawCmd.MarkFlagRequired("client-id")
}
