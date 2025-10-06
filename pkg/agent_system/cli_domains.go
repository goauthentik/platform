package agentsystem

import (
	"github.com/spf13/cobra"
)

var domainsCmd = &cobra.Command{
	Use:   "domains",
	Short: "Configure authentik domains.",
}

func init() {
	rootCmd.AddCommand(domainsCmd)
}
