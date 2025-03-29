package cli

import (
	"context"
	"fmt"
	"os"
	"os/user"
	"strings"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/raw"
	"golang.org/x/crypto/ssh"
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

		host := args[0]
		user := u.Username
		port := "22"
		if strings.Contains(host, "@") {
			_parts := strings.Split(host, "@")
			user = _parts[0]
			host = _parts[1]
		}
		if strings.Contains(host, ":") {
			_parts := strings.Split(host, ":")
			host = _parts[0]
			port = _parts[1]
		}

		config := &ssh.ClientConfig{
			User: fmt.Sprintf("%s@ak-token", user),
			Auth: []ssh.AuthMethod{
				ssh.KeyboardInteractive(func(name, instruction string, questions []string, echos []bool) ([]string, error) {
					fmt.Printf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
					if len(questions) > 0 && questions[0] == "ak-cli-token-prompt:" {
						return []string{cc.AccessToken}, nil
					}
					return []string{}, nil
				}),
			},
			// TODO
			HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		}
		client, err := ssh.Dial("tcp", fmt.Sprintf("%s:%s", host, port), config)
		if err != nil {
			log.Fatal("Failed to dial: ", err)
		}
		defer client.Close()
		// Each ClientConn can support multiple interactive sessions,
		// represented by a Session.
		session, err := client.NewSession()
		if err != nil {
			log.Fatal("Failed to create session: ", err)
		}
		defer session.Close()

		session.Stderr = os.Stderr
		session.Stdout = os.Stdout
		session.Stdin = os.Stdin
		session.Shell()
		<-context.Background().Done()
	},
}

func init() {
	rootCmd.AddCommand(sshCmd)
}
