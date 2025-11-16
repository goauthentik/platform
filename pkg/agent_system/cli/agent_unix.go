//go:build unix

package cli

import (
	"github.com/spf13/cobra"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

func runAgentPlatform(cmd *cobra.Command, args []string) error {
	defer systemlog.Cleanup()
	agent, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: disabledComponents,
	})
	if err != nil {
		return err
	}
	agent.Start()
	return nil
}
