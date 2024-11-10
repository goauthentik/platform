package cmd

import (
	"fmt"
	"io"
	"net/http"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/cfg"
)

// whoamiCmd represents the whoami command
var whoamiCmd = &cobra.Command{
	Use:   "whoami",
	Short: "Check user account details for a given profile",
	Run: func(cmd *cobra.Command, args []string) {
		mgr, err := cfg.Manager()
		if err != nil {
			log.WithError(err).Panic("failed to initialise config manager")
		}
		profile := mustFlag(cmd.Flags().GetString("profile"))
		prof := mgr.Get().Profiles[profile]
		req, err := http.NewRequest("GET", fmt.Sprintf("%s/application/o/userinfo/", prof.AuthentikURL), nil)
		if err != nil {
			log.WithError(err).Panic("failed to create request")
		}
		req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
		res, err := http.DefaultClient.Do(req)
		if err != nil {
			log.WithError(err).Panic("failed to send request")
		}
		if res.StatusCode > 200 {
			log.WithField("status", res.StatusCode).Warning("received status code")
		}
		b, _ := io.ReadAll(res.Body)
		os.Stdout.Write(b)
	},
}

func init() {
	rootCmd.AddCommand(whoamiCmd)
}
