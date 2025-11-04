//go:build windows

package cli

import (
	"os"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	"goauthentik.io/platform/pkg/agent_system/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	windowssvc "goauthentik.io/platform/pkg/platform/windows_svc"
	"golang.org/x/sys/windows/svc"
	"golang.org/x/sys/windows/svc/debug"
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
		err = systemlog.Setup("authentik Sysd")
		if err != nil {
			systemlog.Get().WithError(err).Warning("failed to setup logs")
		}
		return nil
	},
	Run: func(cmd *cobra.Command, args []string) {
		defer systemlog.Cleanup()
		log.SetLevel(log.DebugLevel)
		w := &windowssvc.ServiceWrapper{
			Callback: func() {
				agentsystem.New().Start()
			},
		}
		if isDebug {
			err := debug.Run("ak_sysd", w)
			if err != nil {
				log.Fatalln("Error running service in Service Control mode.")
			}
		} else {
			err := svc.Run("ak_sysd", w)
			if err != nil {
				log.Fatalln("Error running service in Service Control mode.")
			}
		}
	},
}

func init() {
	defaultConfigFile = `C:\Program Files\Authentik Security Inc\sysd\config.json`
	agentCmd.Flags().BoolVarP(&isDebug, "debug", "d", false, "Run in debug mode.")
	rootCmd.AddCommand(agentCmd)
}
