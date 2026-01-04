//go:build unix

package cli

import (
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/sysd"
)

func runAgentPlatform(cmd *cobra.Command, args []string) error {
	agent, err := sysd.New(sysd.SystemAgentOptions{
		DisabledComponents: disabledComponents,
	})
	if err != nil {
		return err
	}
	agent.Start()
	return nil
}
