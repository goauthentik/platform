package cli

import (
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/pb"
)

var systemStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Status about the current session",
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := sysClient()
		if err != nil {
			return err
		}
		sessId, ok := os.LookupEnv("AUTHENTIK_SESSION_ID")
		if !ok {
			return errors.New("current session is not an authentik session")
		}
		res, err := client.SessionStatus(cmd.Context(), &pb.SessionStatusRequest{
			SessionId: sessId,
		})
		if err != nil {
			return err
		}
		fmt.Println(time.Until(res.Expiry.AsTime()).String())
		return nil
	},
}

func init() {
	if _, err := os.Stat(sysSocket); err == nil {
		systemCmd.AddCommand(systemStatusCmd)
	}
}
