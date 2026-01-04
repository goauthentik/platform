package cli

import (
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent/client"
	"goauthentik.io/platform/pkg/cli/auth/vault"
)

var vaultCmd = &cobra.Command{
	Use:   "vault",
	Short: "Generate a JWT for authenticating to HashiCorp Vault.",
	RunE: func(cmd *cobra.Command, args []string) error {
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}

		output := vault.GetCredentials(c, cmd.Context(), vault.CredentialsOpts{
			ClientID:  mustFlag(cmd.Flags().GetString("client-id")),
			Profile:   mustFlag(cmd.Flags().GetString("profile")),
			MountPath: mustFlag(cmd.Flags().GetString("mount-path")),
		})
		_, err = os.Stdout.WriteString(output.ClientToken)
		return err
	},
}

func init() {
	authCmd.AddCommand(vaultCmd)
	vaultCmd.Flags().StringP("client-id", "c", "", "Client ID")
	vaultCmd.Flags().StringP("mount-path", "m", "authentik/", "Vault authentication backend mountpoint")
	_ = vaultCmd.MarkFlagRequired("client-id")
}
