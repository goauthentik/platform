package cli

import (
	"encoding/json"
	"fmt"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/platform/facts"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

var troubleshootFactsCmd = &cobra.Command{
	Use:   "facts",
	Short: "Inspect facts",
	RunE: func(cmd *cobra.Command, args []string) error {
		facts, err := facts.Gather(common.New(log.WithField("cmd", "facts"), cmd.Context()))
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
