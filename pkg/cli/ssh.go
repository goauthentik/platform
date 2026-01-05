package cli

import (
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/cli/ssh"
)

var insecure = false

var sshCmd = &cobra.Command{
	Use:          "ssh",
	Short:        "Establish an SSH connection with `host`.",
	Args:         cobra.MinimumNArgs(1),
	SilenceUsage: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		agentClient, err := client.New(socketPath)
		if err != nil {
			return err
		}

		client, err := ssh.ParseArgs(args)
		if err != nil {
			return err
		}

		client.AgentClient = agentClient
		client.AgentProfile = profile
		client.Insecure = insecure
		return client.Connect()
	},
}

func init() {
	sshCmd.Flags().BoolVarP(&insecure, "insecure", "i", false, "Insecure host-key checking, use with caution!")
	rootCmd.AddCommand(sshCmd)
}
