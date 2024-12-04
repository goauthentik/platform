package cli

import (
	"github.com/cli/oauth"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

// setupCmd represents the setup command
var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Configure authentik CLI",

	Run: func(cmd *cobra.Command, args []string) {
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		appSlug := mustFlag(cmd.Flags().GetString("app"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		mgr := storage.Manager()
		urls := ak.URLsForProfile(storage.ConfigV1Profile{
			AuthentikURL: base,
			AppSlug:      appSlug,
		})

		flow := &oauth.Flow{
			Host: &oauth.Host{
				AuthorizeURL:  urls.AuthorizeURL,
				DeviceCodeURL: urls.DeviceCodeURL,
				TokenURL:      urls.TokenURL,
			},
			ClientID: clientId,
			Scopes:   []string{"openid", "profile", "email", "offline_access"},
		}

		accessToken, err := flow.DetectFlow()
		if err != nil {
			log.WithError(err).Fatal("failed to start device flow")
		}

		profileName := mustFlag(cmd.Flags().GetString("profile"))
		mgr.Get().Profiles[profileName] = storage.ConfigV1Profile{
			AuthentikURL: base,
			AppSlug:      appSlug,
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
	setupCmd.Flags().StringP("app", "s", "authentik-cli", "Slug of the CLI application")
	setupCmd.Flags().StringP("client-id", "i", "authentik-cli", "Client ID")
}
