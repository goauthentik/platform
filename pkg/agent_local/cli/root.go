package cli

import (
	"fmt"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	agent "goauthentik.io/platform/pkg/agent_local"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
)

var (
	isDebug = false
)

var rootCmd = &cobra.Command{
	Use:     "ak-agent",
	Short:   fmt.Sprintf("authentik User Agent v%s", meta.FullVersion()),
	Version: meta.FullVersion(),
	PreRunE: func(cmd *cobra.Command, args []string) error {
		if isDebug {
			log.SetLevel(log.DebugLevel)
		}
		err := systemlog.Setup(pstr.PlatformString{
			// Needs to match event log name in Package.wxs
			Windows: pstr.S("authentik User Service"),
			Linux:   pstr.S("ak-agent"),
		}.ForCurrent())
		if err != nil {
			return err
		}
		return nil
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		a, err := agent.New()
		if err != nil {
			return err
		}
		if isDebug {
			a.StartForeground()
		} else {
			a.Start()
		}
		return nil
	},
}

func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.Flags().BoolVarP(&isDebug, "debug", "d", false, "Run in debug mode.")
}
