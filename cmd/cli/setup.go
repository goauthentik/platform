package cli

import (
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/ak/setup"
)

// setupCmd represents the setup command
var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Configure authentik CLI",

	Run: func(cmd *cobra.Command, args []string) {
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		appSlug := mustFlag(cmd.Flags().GetString("app"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))
		profileName := mustFlag(cmd.Flags().GetString("profile"))

		setup.Setup(setup.Options{
			AuthentikURL: base,
			AppSlug:      appSlug,
			ClientID:     clientId,
			ProfileName:  profileName,
		})
	},
}

func init() {
	rootCmd.AddCommand(setupCmd)
	setupCmd.Flags().StringP("authentik-url", "a", "", "URL to the authentik Instance")
	setupCmd.Flags().StringP("app", "s", setup.DefaultAppSlug, "Slug of the CLI application")
	setupCmd.Flags().StringP("client-id", "i", setup.DefaultClientID, "Client ID")
}
