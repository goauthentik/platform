package cli

import (
	"fmt"
	"os"
	"os/signal"
	"os/user"
	"path"
	"strings"
	"syscall"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/raw"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/knownhosts"
	"golang.org/x/term"
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

		khf, err := DefaultKnownHostsPath()
		if err != nil {
			log.WithError(err).Warning("failed to locate known_hosts")
			return
		}
		kf, err := knownhosts.New(khf)
		if err != nil {
			log.WithError(err).Warning("failed to open known_hosts")
			return
		}

		config := &ssh.ClientConfig{
			User: user,
			Auth: []ssh.AuthMethod{
				ssh.KeyboardInteractive(func(name, instruction string, questions []string, echos []bool) ([]string, error) {
					log.Debugf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
					if len(questions) > 0 && questions[0] == "authentik Password: " {
						return []string{fmt.Sprintf("\u200b%s", cc.AccessToken)}, nil
					}
					ans := []string{}
					for _, q := range questions {
						l, err := ReadPassword(q)
						fmt.Println("")
						if err != nil {
							return ans, err
						}
						ans = append(ans, l)
					}
					return ans, nil
				}),
			},
			HostKeyCallback: kf,
		}
		client, err := ssh.Dial("tcp", fmt.Sprintf("%s:%s", host, port), config)
		if err != nil {
			log.Fatal("Failed to dial: ", err)
		}
		defer client.Close()

		// Create a session for interactive shell
		session, err := client.NewSession()
		if err != nil {
			log.Fatalf("Failed to create session: %v", err)
		}
		defer session.Close()

		// Set up terminal
		session.Stdout = os.Stdout
		session.Stderr = os.Stderr
		session.Stdin = os.Stdin

		// Request a pseudo terminal
		if term.IsTerminal(int(os.Stdin.Fd())) {
			originalState, err := term.MakeRaw(int(os.Stdin.Fd()))
			if err != nil {
				log.Fatalf("Failed to set raw mode: %v", err)
			}
			defer term.Restore(int(os.Stdin.Fd()), originalState)

			width, height, err := term.GetSize(int(os.Stdin.Fd()))
			if err != nil {
				width, height = 80, 24
			}

			if err := session.RequestPty("xterm", height, width, ssh.TerminalModes{}); err != nil {
				log.Fatalf("Failed to request pty: %v", err)
			}
		}

		// Start shell
		if err := session.Shell(); err != nil {
			log.Fatalf("Failed to start shell: %v", err)
		}

		// Wait for session to end
		session.Wait()
	},
}

func init() {
	rootCmd.AddCommand(sshCmd)
}

func ReadPassword(prompt string) (string, error) {
	stdin := int(syscall.Stdin)
	oldState, err := term.GetState(stdin)
	if err != nil {
		return "", err
	}
	defer term.Restore(stdin, oldState)

	sigch := make(chan os.Signal, 1)
	signal.Notify(sigch, os.Interrupt)
	go func() {
		for range sigch {
			term.Restore(stdin, oldState)
			os.Exit(1)
		}
	}()

	fmt.Print(prompt)
	password, err := term.ReadPassword(stdin)
	if err != nil {
		return "", err
	}
	return string(password), nil
}

func DefaultKnownHostsPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return path.Join(home, ".ssh/known_hosts"), err
}
