package cli

import (
	"fmt"
	"os"
	"time"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/pb"
)

var sessionStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Status about the current session",
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := sysClient()
		if err != nil {
			return err
		}
		res, err := client.SessionStatus(cmd.Context(), &pb.SessionStatusRequest{
			SessionId: os.Getenv("AUTHENTIK_SESSION_ID"),
		})
		if err != nil {
			return err
		}
		fmt.Println(time.Until(res.Expiry.AsTime()).String())
		return nil
	},
}

func init() {
	sessionCmd.AddCommand(sessionStatusCmd)
}
