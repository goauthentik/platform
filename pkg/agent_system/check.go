package agentsystem

import (
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent_system/check"
)

var checkCmd = &cobra.Command{
	Use:   "check",
	Short: "Check the status of the authentik system agent",
	RunE: func(cmd *cobra.Command, args []string) error {
		return check.RunChecks(cmd.Context())
	},
}

func init() {
	rootCmd.AddCommand(checkCmd)
}
