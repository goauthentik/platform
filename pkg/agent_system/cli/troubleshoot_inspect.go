package cli

import (
	"fmt"

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
		cap, err := sc.Capabilities(cmd.Context(), &emptypb.Empty{})
		if err != nil {
			return err
		}

		t := tree.New().Root(r.Bucket).Enumerator(tree.RoundedEnumerator)
		tui.AddNodeToTree(t, "Capabilities", cap, tui.KeyStyle, tui.ValueStyle)
		fmt.Println(renderInspectAsTree(r, t))
		return nil
	},
}

func renderInspectAsTree(r *pb.TroubleshootInspectResponse, t *tree.Tree) string {
	// Add each key-value pair to the tree
	for k, v := range r.Kv {
		tui.AddNodeToTree(t, tui.KeyStyle.Render(k), v, tui.KeyStyle, tui.ValueStyle)
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
