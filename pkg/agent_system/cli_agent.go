//go:build !windows

package agentsystem

import (
	"os"
	"runtime"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
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
		New().Start()
	},
}

func init() {
	defaultConfigFile = "/etc/authentik/config.json"
	if runtime.GOOS == "darwin" {
		defaultConfigFile = "/opt/authentik/config/config.json"
	}
	rootCmd.AddCommand(agentCmd)
}
