package cli

import (
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/vault"
)

var vaultCmd = &cobra.Command{
	Use:   "vault",
	Short: "Generate a JWT for authenticating to HashiCorp Vault.",
	Run: func(cmd *cobra.Command, args []string) {
		output := vault.GetCredentials(cmd.Context(), vault.CredentialsOpts{
			ClientID:  mustFlag(cmd.Flags().GetString("client-id")),
			Profile:   mustFlag(cmd.Flags().GetString("profile")),
			MountPath: mustFlag(cmd.Flags().GetString("mount-path")),
		})
		if output == nil {
			return
		}
		_, err := os.Stdout.WriteString(output.ClientToken)
		if err != nil {
			log.WithError(err).Warning("failed to write token")
		}
	},
}

func init() {
	authCmd.AddCommand(vaultCmd)
	vaultCmd.Flags().StringP("client-id", "c", "", "Client ID")
	vaultCmd.Flags().StringP("mount-path", "m", "authentik/", "Vault authentication backend mountpoint")
	_ = vaultCmd.MarkFlagRequired("client-id")
}
