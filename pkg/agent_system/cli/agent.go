package cli

import (
	"os"

	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
)

var isDebug = false

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
		err = systemlog.Setup(pstr.PlatformString{
			Windows: pstr.S("authentik Sysd"),
			Linux:   pstr.S("ak-sysd"),
		}.ForCurrent())
		if err != nil {
			return err
		}
		return nil
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		return runAgentPlatform(cmd, args)
	},
}

func init() {
	agentCmd.Flags().BoolVarP(&isDebug, "debug", "d", false, "Run in debug mode.")
	rootCmd.AddCommand(agentCmd)
}
