package cli

import (
	"fmt"
	"strings"

	"github.com/spf13/cobra"
	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/agent_system/config"
)

var troubleshootInspectCmd = &cobra.Command{
	Use:   "inspect",
	Short: "Inspect state",
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return agentPrecheck()
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		depth := 0
		return config.State().View(func(tx *bbolt.Tx) error {
			return tx.ForEach(func(name []byte, b *bbolt.Bucket) error {
				inspectBucket(name, b, depth)
				return nil
			})
		})
	},
}

func inspectBucket(name []byte, b *bbolt.Bucket, depth int) {
	fmt.Printf("%sBucket '%s':\n", strings.Repeat("\t", depth), string(name))
	depth += 1
	seenKeys := map[string]struct{}{}
	_ = b.ForEachBucket(func(k []byte) error {
		bb := b.Bucket(k)
		if bb != nil {
			seenKeys[string(k)] = struct{}{}
			inspectBucket(k, bb, depth)
		}
		return nil
	})
	_ = b.ForEach(func(k, v []byte) error {
		vv := string(v)
		if _, seen := seenKeys[vv]; seen {
			return nil
		}
		fmt.Printf("%sKey '%s' => '%s'\n", strings.Repeat("\t", depth), string(k), vv)
		return nil
	})
}

func init() {
	troubleshootCmd.AddCommand(troubleshootInspectCmd)
}
