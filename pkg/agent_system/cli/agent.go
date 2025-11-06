//go:build !windows

package cli

import (
	"os"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	"goauthentik.io/platform/pkg/agent_system/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

var agentCmd = &cobra.Command{
	Use:          "agent",
	Short:        "Run the authentik system agent",
	SilenceUsage: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		err := agentPrecheck()
		if err != nil {
			return err
		}
		if _, err := os.Stat(config.Manager().Get().RuntimeDir); err != nil {
			return errors.Wrap(err, "failed to check runtime directory")
		}
		err = systemlog.Setup("sysd")
		if err != nil {
			return err
		}
		return nil
	},
	Run: func(cmd *cobra.Command, args []string) {
		defer systemlog.Cleanup()
		log.SetLevel(log.DebugLevel)
		agentsystem.New().Start()
	},
}

func init() {
	rootCmd.AddCommand(agentCmd)
}
