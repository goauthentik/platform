package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

// whoamiCmd represents the whoami command
var whoamiCmd = &cobra.Command{
	Use:   "whoami",
	Short: "Check user account details for a given profile",
	Run: func(cmd *cobra.Command, args []string) {
		mgr := storage.Manager()
		profile := mustFlag(cmd.Flags().GetString("profile"))
		prof := mgr.Get().Profiles[profile]
		req, err := http.NewRequest("GET", ak.URLsForProfile(prof).UserInfo, nil)
		if err != nil {
			log.WithError(err).Panic("failed to create request")
			return
		}
		req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
		res, err := http.DefaultClient.Do(req)
		if err != nil {
			log.WithError(err).Panic("failed to send request")
			return
		}
		if res.StatusCode > 200 {
			log.WithField("status", res.StatusCode).Warning("received status code")
			return
		}
		var m interface{}
		err = json.NewDecoder(res.Body).Decode(&m)
		if err != nil {
			log.WithError(err).Warning("failed to parse JSON")
			return
		}
		b, err := json.MarshalIndent(m, "", "\t")
		if err != nil {
			log.WithError(err).Warning("failed to render JSON")
			return
		}
		os.Stdout.Write(b)
	},
}

func init() {
	rootCmd.AddCommand(whoamiCmd)
}
