package agentsystem

import (
	"fmt"
	"os"

	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/storage"
)

var rootCmd = &cobra.Command{
	Use:   "ak-sys",
	Short: fmt.Sprintf("authentik System Agent v%s", storage.FullVersion()),
}

func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func agentPrecheck() error {
	if os.Getuid() != 0 {
		return errors.New("authentik system agent must run as root")
	}
	if _, err := os.Stat(config.Path); err != nil {
		return errors.Wrap(err, "failed to check config file")
	}
	config.Load()
	if _, err := os.Stat(config.Get().RuntimeDir()); err != nil {
		return errors.Wrap(err, "failed to check runtime directory")
	}
	return nil
}
