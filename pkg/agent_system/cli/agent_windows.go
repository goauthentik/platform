//go:build windows

package cli

import (
	"github.com/spf13/cobra"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	windowssvc "goauthentik.io/platform/pkg/platform/windows_svc"
	"golang.org/x/sys/windows/svc"
	"golang.org/x/sys/windows/svc/debug"
)

func runAgentPlatform(cmd *cobra.Command, args []string) error {
	agent, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: disabledComponents,
	})
	if err != nil {
		return err
	}
	w := &windowssvc.ServiceWrapper{
		Callback: agent.Start,
	}
	if isDebug {
		return debug.Run("ak_sysd", w)
	} else {
		return svc.Run("ak_sysd", w)
	}
}
