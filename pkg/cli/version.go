package cli

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
	lclient "goauthentik.io/platform/pkg/agent_local/client"
	sclient "goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/shared/tui"
	"google.golang.org/protobuf/types/known/emptypb"
)

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Version of authentik Agent components",
	Run: func(cmd *cobra.Command, args []string) {
		version := []string{
			fmt.Sprintf("authentik Agent CLI: %s", meta.FullVersion()),
			fmt.Sprintf("Agent: %s", getUserAgentVersion(cmd.Context())),
			fmt.Sprintf("System: %s", getSystemAgentVersion(cmd.Context())),
		}
		for _, v := range version {
			fmt.Println(tui.InlineStyle().Render(v))
		}
	},
}

func getUserAgentVersion(ctx context.Context) string {
	c, err := lclient.New(socketPath)
	if err != nil {
		return err.Error()
	}
	r, err := c.Ping(ctx, &emptypb.Empty{})
	if err != nil {
		return err.Error()
	}
	return r.Version
}

func getSystemAgentVersion(ctx context.Context) string {
	c, err := sclient.NewDefault()
	if err != nil {
		return err.Error()
	}
	r, err := c.Ping(ctx, &emptypb.Empty{})
	if err != nil {
		return err.Error()
	}
	return r.Version
}

func init() {
	rootCmd.AddCommand(versionCmd)
}
