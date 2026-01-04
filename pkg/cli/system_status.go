package cli

import (
	"errors"
	"fmt"
	"os"
	"time"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/sysd/client"
	"goauthentik.io/platform/pkg/sysd/types"
)

var systemStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Status about the current session",
	RunE: func(cmd *cobra.Command, args []string) error {
		client, err := client.NewDefault()
		if err != nil {
			return err
		}
		defer func() {
			err := client.Close()
			if err != nil {
				log.WithError(err).Warning("Failed to cleanup client")
			}
		}()
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
	if _, err := os.Stat(types.GetSysdSocketPath(types.SocketIDDefault).ForCurrent()); err == nil {
		systemCmd.AddCommand(systemStatusCmd)
	}
}
