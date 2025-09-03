package cli

import (
	"github.com/spf13/cobra"
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Configure authentik CLI",
}

func init() {
	rootCmd.AddCommand(configCmd)
}
