package cli

import (
	"github.com/spf13/cobra"
)

var authCmd = &cobra.Command{
	Use:   "auth",
	Short: "Commands for authenticating with different CLI applications.",
}

func init() {
	rootCmd.AddCommand(authCmd)
}
