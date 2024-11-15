package cli

import (
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent"
)

// agentCmd represents the agent command
var agentCmd = &cobra.Command{
	Use:   "agent",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		a, err := agent.New()
		if err != nil {
			log.WithError(err).Warning("failed to start agent")
		}
		a.Start()
	},
}

func init() {
	rootCmd.AddCommand(agentCmd)
}
