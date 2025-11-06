package cli

import (
	"errors"
	"fmt"
	"net"
	"os"

	"github.com/bramvdbogaerde/go-scp"
	"github.com/google/uuid"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/client"
)

var scpCmd = &cobra.Command{
	Use:          "scp",
	Short:        "Copy files to and from `host`.",
	Args:         cobra.MinimumNArgs(2),
	SilenceUsage: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		uid := uuid.New().String()
		remoteSocketPath := fmt.Sprintf("/var/run/authentik/agent-%s.sock", uid)

		client, err := SSHClient(SSHClientOptions{
			RawHost:          args[1],
			Profile:          profile,
			AgentClient:      c,
			Context:          cmd.Context(),
			RemoteSocketPath: remoteSocketPath,
		})
		if err != nil {
			return err
		}
		defer func() {
			err := client.Close()
			if err != nil && !errors.Is(err, net.ErrClosed) {
				log.WithError(err).Warning("Failed to close client")
			}
		}()

		f, err := os.Open(args[0])
		if err != nil {
			return err
		}

		scpClient, err := scp.NewClientBySSH(client)
		if err != nil {
			return err
		}

		return scpClient.CopyFromFile(cmd.Context(), *f, "/home/server/test.txt", "0655")
	},
}

func init() {
	scpCmd.Flags().BoolVarP(&insecure, "insecure", "i", false, "Insecure host-key checking, use with caution!")
	rootCmd.AddCommand(scpCmd)
}
