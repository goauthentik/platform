package cli

import (
	"fmt"
	"os"
	"os/exec"
	"os/user"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/raw"
)

var sshCmd = &cobra.Command{
	Use:   "ssh",
	Short: "Establish an SSH connection with `host`.",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		profile := mustFlag(cmd.Flags().GetString("profile"))

		u, err := user.Current()
		if err != nil {
			log.WithError(err).Warning("failed to get user")
			os.Exit(1)
		}

		cc := raw.GetCredentials(cmd.Context(), raw.CredentialsOpts{
			Profile:  profile,
			ClientID: "authentik-pam",
		})
		fmt.Println(cc.AccessToken)

		proc := exec.Command("ssh", "-l", fmt.Sprintf("%s@ak-token", u.Name), args[0])
		proc.Env = append(proc.Env, "")
		proc.Stderr = os.Stderr
		proc.Stdout = os.Stdout
		proc.Stdin = os.Stdin
		proc.Start()
		proc.Wait()
		os.Exit(proc.ProcessState.ExitCode())
	},
}

func init() {
	rootCmd.AddCommand(sshCmd)
}
