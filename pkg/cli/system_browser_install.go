package cli

import (
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/browser_native_messaging"
)

var systemBrowserInstallCmd = &cobra.Command{
	Use:   "browser-install",
	Short: "Install browser host",
	RunE: func(cmd *cobra.Command, args []string) error {
		return browser_native_messaging.Install()
	},
}

func init() {
	systemCmd.AddCommand(systemBrowserInstallCmd)
}
