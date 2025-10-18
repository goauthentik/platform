package cli

import (
	"github.com/spf13/cobra"
)

var systemCmd = &cobra.Command{
	Use:   "system",
	Short: "Commands for interacting with authentik sessions.",
}

func init() {
	rootCmd.AddCommand(systemCmd)
}
