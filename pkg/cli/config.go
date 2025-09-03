package cli

import (
	"errors"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/cli/client"
	"goauthentik.io/cli/pkg/cli/setup"
	"goauthentik.io/cli/pkg/pb"
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

		cfg, err := setup.Setup(setup.Options{
			AuthentikURL: base,
			AppSlug:      appSlug,
			ClientID:     clientId,
			ProfileName:  profileName,
		})
		if err != nil {
			return err
		}
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		res, err := c.Setup(cmd.Context(), &pb.SetupRequest{
			Header: &pb.RequestHeader{
				Profile: profileName,
			},
			AuthentikUrl: cfg.AuthentikURL,
			AppSlug:      cfg.AppSlug,
			ClientId:     cfg.ClientID,
			AccessToken:  cfg.AccessToken,
			RefreshToken: cfg.RefreshToken,
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
	rootCmd.AddCommand(setupCmd)
	setupCmd.Flags().StringP("authentik-url", "a", "", "URL to the authentik Instance")
	setupCmd.Flags().StringP("app", "d", setup.DefaultAppSlug, "Slug of the CLI application")
	setupCmd.Flags().StringP("client-id", "i", setup.DefaultClientID, "Client ID")
}
