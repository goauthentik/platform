package cli

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/shared/tui"
	"google.golang.org/protobuf/types/known/emptypb"
)

var versionCmd = &cobra.Command{
	Use:          "version",
	Short:        "Version of authentik Agent components",
	SilenceUsage: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		rv, err := getSystemAgentVersion(cmd.Context())
		if err != nil {
			return err
		}
		version := []string{
			fmt.Sprintf("authentik System Agent: %s", meta.FullVersion()),
			fmt.Sprintf("System: %s", rv),
		}
		for _, v := range version {
			fmt.Println(tui.InlineStyle().Render(v))
		}
		return nil
	},
}

func getSystemAgentVersion(ctx context.Context) (string, error) {
	c, err := client.NewDefault()
	if err != nil {
		return "", err
	}
	r, err := c.Ping(ctx, &emptypb.Empty{})
	if err != nil {
		return "", err
	}
	return r.Version, nil
}

func init() {
	rootCmd.AddCommand(versionCmd)
}
