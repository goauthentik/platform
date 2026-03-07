package cli

import (
	"fmt"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/platform/facts"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/shared/tui"
)

var troubleshootFactsCmd = &cobra.Command{
	Use:   "facts",
	Short: "Inspect facts",
	RunE: func(cmd *cobra.Command, args []string) error {
		facts, err := facts.Gather(common.New(log.WithField("cmd", "facts"), cmd.Context()))
		if err != nil {
			return err
		}
		m, err := tui.AnyToMap(facts)
		if err != nil {
			return err
		}
		fmt.Print(tui.RenderMapAsTree(m, "Facts:"))
		return nil
	},
}

func init() {
	troubleshootCmd.AddCommand(troubleshootFactsCmd)
}
