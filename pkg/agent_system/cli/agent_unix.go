//go:build unix

package cli

import (
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

func runAgentPlatform(cmd *cobra.Command, args []string) error {
	defer systemlog.Cleanup()
	log.SetLevel(log.DebugLevel)
	agent, err := agentsystem.New()
	if err != nil {
		return err
	}
	agent.Start()
	return nil
}
