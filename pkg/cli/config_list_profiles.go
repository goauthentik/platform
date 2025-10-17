package cli

import (
	"encoding/json"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/cli/client"
	"google.golang.org/protobuf/types/known/emptypb"
)

var listProfilesCmd = &cobra.Command{
	Use:   "list-profiles",
	Short: "List profiles",
	RunE: func(cmd *cobra.Command, args []string) error {
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}

		res, err := c.ListProfiles(cmd.Context(), &emptypb.Empty{})
		if err != nil {
			return err
		}
		err = json.NewEncoder(os.Stdout).Encode(res.Profiles)
		if err != nil {
			log.WithError(err).Warning("failed to write raw credentials")
			return err
		}
		return nil
	},
}

func init() {
	configCmd.AddCommand(listProfilesCmd)
}
