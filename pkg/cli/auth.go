package cli

import (
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
)

var authCmd = &cobra.Command{
	Use:   "auth",
	Short: "Commands for authenticating with different CLI applications.",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		// the `auth` group of commands are usually used within other CLI applications
		// that read our stdout, and stderr (which we log to) is forwarded to the user
		// we only want to do that for errors
		log.SetLevel(log.ErrorLevel)
	},
}

func init() {
	rootCmd.AddCommand(authCmd)
}
