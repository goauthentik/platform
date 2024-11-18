package cli

import (
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/vault"
)

func envDefault(env string, def string) string {
	if v, ok := os.LookupEnv(env); ok {
		return v
	}
	return def
}

var vaultAuthGetCmd = &cobra.Command{
	Use: "get",
	Run: func(cmd *cobra.Command, args []string) {
		output := vault.GetCredentials(cmd.Context(), vault.CredentialsOpts{
			Profile:   envDefault("AUTHENTIK_VAULT_PROFILE", "default"),
			ClientID:  os.Getenv("AUTHENTIK_VAULT_CLIENTID"),
			MountPath: envDefault("AUTHENTIK_VAULT_MOUNTPATH", "oidc"),
		})
		if output == nil {
			return
		}
		os.Stdout.WriteString(output.ClientToken)
	},
}

func init() {
	vaultCmd.AddCommand(vaultAuthGetCmd)
}
