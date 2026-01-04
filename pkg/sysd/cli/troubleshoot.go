package cli

import (
	"github.com/spf13/cobra"
)

var troubleshootCmd = &cobra.Command{
	Use:   "troubleshoot",
	Short: "Troubleshooting commands",
}

func init() {
	rootCmd.AddCommand(troubleshootCmd)
}
