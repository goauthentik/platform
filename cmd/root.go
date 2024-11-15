package cmd

import (
	"fmt"
	"os"
	"path"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/storage"
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
	Short: fmt.Sprintf("authentik CLI v%s", storage.FullVersion()),
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
	var err error
	if path.Base(os.Args[0]) == "ak-vault" {
		err = vaultCmd.Execute()
	} else {
		err = rootCmd.Execute()
	}
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.PersistentFlags().BoolP("verbose", "v", false, "Enable debug logging")
	rootCmd.PersistentFlags().StringP("profile", "n", "default", "A name for the profile")
}
