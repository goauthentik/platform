package cli

import (
	"errors"
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/cli/setup"
	"goauthentik.io/platform/pkg/pb"
)

// setupCmd represents the setup command
var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Configure authentik CLI",
	RunE: func(cmd *cobra.Command, args []string) error {
		profileName := mustFlag(cmd.Flags().GetString("profile"))
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		appSlug := mustFlag(cmd.Flags().GetString("app"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		accessToken, refreshToken := "", ""
		if at, aset := os.LookupEnv("AK_CLI_ACCESS_TOKEN"); aset {
			accessToken = at
			refreshToken = os.Getenv("AK_CLI_REFRESH_TOKEN")
		} else {
			cfg, err := setup.Setup(setup.Options{
				AuthentikURL: base,
				AppSlug:      appSlug,
				ClientID:     clientId,
				ProfileName:  profileName,
			})
			if err != nil {
				return err
			}
			accessToken = cfg.AccessToken
			refreshToken = cfg.RefreshToken
		}

		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		res, err := c.Setup(cmd.Context(), &pb.SetupRequest{
			Header: &pb.RequestHeader{
				Profile: profileName,
			},
			AuthentikUrl: base,
			AppSlug:      appSlug,
			ClientId:     clientId,
			AccessToken:  accessToken,
			RefreshToken: refreshToken,
		})
		if err != nil {
			return err
		}
		if !res.Header.Successful {
			return errors.New("setup not successful")
		}
		return nil
	},
}

func init() {
	configCmd.AddCommand(setupCmd)
	setupCmd.Flags().StringP("authentik-url", "a", "", "URL to the authentik Instance")
	setupCmd.Flags().StringP("app", "d", setup.DefaultAppSlug, "Slug of the CLI application")
	setupCmd.Flags().StringP("client-id", "i", setup.DefaultClientID, "Client ID")
}
