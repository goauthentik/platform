//go:build !windows
// +build !windows

package agentsystem

import (
	"os"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/systemlog"
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
			systemlog.Get().WithError(err).Warning("failed to setup logs")
		}
		return nil
	},
	Run: func(cmd *cobra.Command, args []string) {
		defer systemlog.Cleanup()
		log.SetLevel(log.DebugLevel)
		New().Start()
	},
}

func init() {
	defaultConfigFile = "/etc/authentik/config.json"
	rootCmd.AddCommand(agentCmd)
}
