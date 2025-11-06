package cli

import (
	"fmt"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/storage/state"
)

var troubleshootInspectCmd = &cobra.Command{
	Use:   "inspect",
	Short: "Inspect state",
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return agentPrecheck()
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		sst, err := state.Open(types.StatePath().ForCurrent(), &bbolt.Options{
			Timeout:  1 * time.Second,
			ReadOnly: true,
		})
		if err != nil {
			return err
		}
		depth := 0
		return sst.View(func(tx *bbolt.Tx) error {
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
	_ = b.ForEach(func(k, v []byte) error {
		vv := string(v)
		if len(vv) > 16 {
			vv = vv[:16]
		}
		fmt.Printf("%sKey '%s' => '%s'\n", strings.Repeat("\t", depth), string(k), vv)
		return nil
	})
	_ = b.ForEachBucket(func(k []byte) error {
		bb := b.Bucket(k)
		if bb != nil {
			inspectBucket(k, bb, depth)
		}
		return nil
	})
}

func init() {
	troubleshootCmd.AddCommand(troubleshootInspectCmd)
}
