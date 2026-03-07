package cli

import (
	"encoding/json"
	"fmt"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/tui"
)

var whoamiCmd = &cobra.Command{
	Use:   "whoami",
	Short: "Check user account details for a given profile",
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		res, err := c.WhoAmI(cmd.Context(), &pb.WhoAmIRequest{
			Header: &pb.RequestHeader{
				Profile: profile,
			},
		})
		if err != nil {
			return err
		}
		if !res.Header.Successful {
			log.Warning("received status code")
			return nil
		}
		var m map[string]any
		err = json.Unmarshal([]byte(res.Body), &m)
		if err != nil {
			log.WithError(err).Warning("failed to parse JSON")
			return err
		}
		if jsonMode {
			return tui.JSON(m)
		} else {
			fmt.Print(tui.RenderMapAsTree(m, "User Information:"))
		}
		return nil
	},
}

func init() {
	rootCmd.AddCommand(whoamiCmd)
}
