package cli

import (
	"fmt"
	"io"
	"net"
	"os"
	"os/signal"
	"os/user"
	"path"
	"strings"
	"syscall"

	"github.com/google/uuid"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent_local/types"
	"goauthentik.io/cli/pkg/cli/auth/raw"
	"goauthentik.io/cli/pkg/cli/client"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/knownhosts"
	"golang.org/x/term"
)

var sshCmd = &cobra.Command{
	Use:   "ssh",
	Short: "Establish an SSH connection with `host`.",
	Args:  cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}

		u, err := user.Current()
		if err != nil {
			log.WithError(err).Warning("failed to get user")
			return err
		}

		cc := raw.GetCredentials(c, cmd.Context(), raw.CredentialsOpts{
			Profile:  profile,
			ClientID: "authentik-pam",
		})

		uh, hostPort, err := net.SplitHostPort(args[0])
		if err != nil {
			return err
		}
		user, host := func(s string) (string, string) {
			if i := strings.IndexByte(s, '@'); i >= 0 {
				return s[:i], s[i+1:]
			}
			return u.Username, s
		}(uh)

		khf, err := DefaultKnownHostsPath()
		if err != nil {
			log.WithError(err).Warning("failed to locate known_hosts")
			return err
		}
		_, err = knownhosts.New(khf)
		if err != nil {
			log.WithError(err).Warning("failed to open known_hosts")
			return err
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
			HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		}
		client, err := ssh.Dial("tcp", net.JoinHostPort(host, hostPort), config)
		if err != nil {
			log.Fatal("Failed to dial: ", err)
		}
		defer client.Close()

		ForwardAgentSocket(client)

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
		return session.Wait()
	},
}

func init() {
	rootCmd.AddCommand(sshCmd)
}

func ForwardAgentSocket(client *ssh.Client) {
	remoteSocket := fmt.Sprintf("/var/run/authentik/%s.sock", uuid.New())
	localSocket := types.GetAgentSocketPath()
	remoteListener, err := client.Listen("unix", remoteSocket)
	if err != nil {
		log.Fatalf("remote listen on %s failed: %v", remoteSocket, err)
	}
	defer remoteListener.Close()
	log.Printf("remote listening %s → local %s", remoteSocket, localSocket)

	for {
		remoteConn, err := remoteListener.Accept()
		if err != nil {
			log.Printf("remote Accept error: %v", err)
			continue
		}
		go func(rc net.Conn) {
			defer rc.Close()
			// Dial the local unix socket
			lc, err := net.Dial("unix", localSocket)
			if err != nil {
				log.Printf("local dial %s failed: %v", localSocket, err)
				return
			}
			defer lc.Close()

			// Pipe both ways
			done := make(chan struct{}, 2)
			go func() {
				io.Copy(rc, lc)
				done <- struct{}{}
			}()
			go func() {
				io.Copy(lc, rc)
				done <- struct{}{}
			}()
			<-done
		}(remoteConn)
	}
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
