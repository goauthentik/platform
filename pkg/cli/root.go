package cli

import (
	"fmt"
	"os"
	"path"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/storage"
)

var (
	socketPath string
)

func mustFlag[T any](res T, err error) T {
	if err != nil {
		log.WithError(err).Panic("Missing required argument")
	}
	return res
}

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
	arg0 := path.Base(os.Args[0])
	switch arg0 {
	case "ak-vault":
		err = vaultCmd.Execute()
	case "ak-browser-support":
		err = browserSupportCmd.Execute()
	default:
		err = rootCmd.Execute()
	}
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	defaultSocketPath := types.GetAgentSocketPath()
	rootCmd.PersistentFlags().BoolP("verbose", "v", false, "Enable debug logging")
	rootCmd.PersistentFlags().StringP("profile", "n", "default", "A name for the profile")
	rootCmd.PersistentFlags().StringVarP(&socketPath, "socket", "s", defaultSocketPath.ForCurrent(), "Socket the agent is listening on")
}
