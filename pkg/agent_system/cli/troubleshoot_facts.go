package cli

import (
	"encoding/json"
	"fmt"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/platform/facts"
)

var troubleshootFactsCmd = &cobra.Command{
	Use:   "facts",
	Short: "Inspect facts",
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return agentPrecheck()
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		facts, err := facts.Gather(log.WithField("cmd", "facts"))
		if err != nil {
			return err
		}
		b, err := json.MarshalIndent(facts, "", "\t")
		if err != nil {
			log.WithError(err).Warning("failed to render JSON")
			return err
		}
		fmt.Println(string(b))
		return nil
	},
}

func init() {
	troubleshootCmd.AddCommand(troubleshootFactsCmd)
}
