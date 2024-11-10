package cmd

import (
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
)

func mustFlag[T any](res T, err error) T {
	if err != nil {
		log.WithError(err).Panic("Missing required argument")
	}
	return res
}

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   "ak",
	Short: "authentik CLI",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		verbose := mustFlag(cmd.Flags().GetBool("verbose"))
		if verbose {
			log.SetLevel(log.DebugLevel)
		}
		// Log to stderr especially for `ak auth ...` commands
		log.SetOutput(os.Stderr)
	},
}

func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.PersistentFlags().BoolP("verbose", "v", false, "Enable debug logging")
	rootCmd.PersistentFlags().StringP("profile", "n", "default", "A name for the profile")
}
