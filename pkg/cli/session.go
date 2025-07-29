package cli

import (
	"github.com/spf13/cobra"
)

var sessionCmd = &cobra.Command{
	Use:   "session",
	Short: "Commands for interacting with authentik sessions.",
}

func init() {
	rootCmd.AddCommand(sessionCmd)
}
