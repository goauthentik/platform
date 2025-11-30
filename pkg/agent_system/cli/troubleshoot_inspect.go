package cli

import (
	"fmt"
	"strings"

	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

var troubleshootInspectCmd = &cobra.Command{
	Use:   "inspect",
	Short: "Inspect state",
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return agentPrecheck()
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		sc, err := client.NewCtrl()
		if err != nil {
			return errors.Wrap(err, "failed to connect to ctrl")
		}
		r, err := sc.TroubleshootInspect(cmd.Context(), &emptypb.Empty{})
		if err != nil {
			return err
		}
		inspectBucket(r, 0)
		return nil
	},
}

func inspectBucket(r *pb.TroubleshootInspectResponse, depth int) {
	fmt.Printf("%sBucket '%s':\n", strings.Repeat("\t", depth), r.Bucket)
	depth += 1
	for k, v := range r.Kv {
		fmt.Printf("%sKey '%s' => '%s'\n", strings.Repeat("\t", depth), k, v)
	}
	for _, ch := range r.Children {
		inspectBucket(ch, depth+1)
	}
}

func init() {
	troubleshootCmd.AddCommand(troubleshootInspectCmd)
}
