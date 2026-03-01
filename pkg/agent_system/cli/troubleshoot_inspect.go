package cli

import (
	"fmt"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/tree"
	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/tui"
	"google.golang.org/protobuf/types/known/emptypb"
)

var troubleshootInspectCmd = &cobra.Command{
	Use:   "inspect",
	Short: "Inspect state",
	RunE: func(cmd *cobra.Command, args []string) error {
		sc, err := client.NewCtrl()
		if err != nil {
			return errors.Wrap(err, "failed to connect to ctrl")
		}
		r, err := sc.TroubleshootInspect(cmd.Context(), &emptypb.Empty{})
		if err != nil {
			return err
		}

		t := tree.New().Root(r.Bucket).Enumerator(tree.RoundedEnumerator)
		fmt.Println(renderInspectAsTree(r, t))
		return nil
	},
}

func renderInspectAsTree(r *pb.TroubleshootInspectResponse, t *tree.Tree) string {
	// Create styles for different types
	keyStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("6"))
	valueStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("2"))

	// Add each key-value pair to the tree
	for k, v := range r.Kv {
		tui.AddNodeToTree(t, keyStyle.Render(k), v, keyStyle, valueStyle)
	}
	for _, ch := range r.Children {
		cht := tree.New().Root(ch.Bucket)
		renderInspectAsTree(ch, cht)
		t.Child(cht)
	}

	return t.String()
}

func init() {
	troubleshootCmd.AddCommand(troubleshootInspectCmd)
}
