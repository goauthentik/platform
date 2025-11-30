package cli

import (
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/check"
)

var troubleshootCheckCmd = &cobra.Command{
	Use:   "check",
	Short: "Check the status of the authentik system agent",
	RunE: func(cmd *cobra.Command, args []string) error {
		return check.RunChecks(cmd.Context())
	},
}

func init() {
	troubleshootCmd.AddCommand(troubleshootCheckCmd)
}
