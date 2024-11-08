package cmd

import (
	"fmt"

	"github.com/cli/oauth"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/cfg"
)

// setupCmd represents the setup command
var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Configure authentik CLI",

	Run: func(cmd *cobra.Command, args []string) {
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		mgr, err := cfg.Manager()
		if err != nil {
			log.WithError(err).Panic("failed to initialise config manager")
		}

		flow := &oauth.Flow{
			Host: &oauth.Host{
				AuthorizeURL:  fmt.Sprintf("%s/application/o/authorize/", base),
				DeviceCodeURL: fmt.Sprintf("%s/application/o/device/", base),
				TokenURL:      fmt.Sprintf("%s/application/o/token/", base),
			},
			ClientID: clientId,
			Scopes:   []string{"openid", "profile", "email", "offline_access"},
		}

		accessToken, err := flow.DetectFlow()
		if err != nil {
			log.WithError(err).Fatal("failed to start device flow")
		}

		profileName := mustFlag(cmd.Flags().GetString("profile"))
		mgr.Get().Profiles[profileName] = cfg.ConfigV1Profile{
			AuthentikURL: base,
			ClientID:     clientId,
			AccessToken:  accessToken.Token,
			RefreshToken: accessToken.RefreshToken,
		}
		err = mgr.Save()
		if err != nil {
			log.WithError(err).Warning("failed to save config")
		}
	},
}

func init() {
	rootCmd.AddCommand(setupCmd)
	setupCmd.Flags().StringP("authentik-url", "a", "", "URL to the authentik Instance")
	setupCmd.Flags().StringP("client-id", "i", "authentik-cli", "Client ID")
}
