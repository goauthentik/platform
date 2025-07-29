package cli

import (
	"encoding/hex"
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
	"goauthentik.io/cli/pkg/pb"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/knownhosts"
	"golang.org/x/term"
	"google.golang.org/protobuf/proto"
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
			return err
		}
		kf, err := knownhosts.New(khf)
		if err != nil {
			log.WithError(err).Warning("failed to open known_hosts")
			return err
		}

		uid := uuid.New().String()
		remoteSocketPath := fmt.Sprintf("/var/run/authentik/agent-%s.sock", uid)

		config := &ssh.ClientConfig{
			User: user,
			Auth: []ssh.AuthMethod{
				ssh.KeyboardInteractive(func(name, instruction string, questions []string, echos []bool) ([]string, error) {
					log.Debugf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
					if len(questions) > 0 && questions[0] == "authentik Password: " {
						return []string{FormatToken(cc, remoteSocketPath)}, nil
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
		client, err := ssh.Dial("tcp", net.JoinHostPort(host, port), config)
		if err != nil {
			log.WithError(err).Fatal("Failed to dial")
		}
		defer func() {
			err := client.Close()
			log.WithError(err).Warning("Failed to close client")
		}()

		go ForwardAgentSocket(remoteSocketPath, client)

		// Create a session for interactive shell
		session, err := client.NewSession()
		if err != nil {
			log.WithError(err).Fatal("Failed to create session")
		}
		defer func() {
			err := session.Close()
			if err != nil {
				log.WithError(err).Warning("Failed to close session")
			}
		}()

		// Set up terminal
		session.Stdout = os.Stdout
		session.Stderr = os.Stderr
		session.Stdin = os.Stdin

		// Request a pseudo terminal
		if term.IsTerminal(int(os.Stdin.Fd())) {
			originalState, err := term.MakeRaw(int(os.Stdin.Fd()))
			if err != nil {
				log.WithError(err).Fatal("Failed to set raw mode")
			}
			defer func() {
				err := term.Restore(int(os.Stdin.Fd()), originalState)
				if err != nil {
					log.WithError(err).Warn("Failed to restore terminal state")
				}
			}()

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

func FormatToken(cc *raw.RawCredentialOutput, rtp string) string {
	msg := pb.PAMAuthentication{
		Token:       cc.AccessToken,
		LocalSocket: rtp,
	}
	rv, err := proto.Marshal(&msg)
	if err != nil {
		panic(err)
	}
	return fmt.Sprintf("\u200b%s", hex.EncodeToString(rv))
}

func ForwardAgentSocket(remoteSocket string, client *ssh.Client) {
	localSocket := types.GetAgentSocketPath()
	remoteListener, err := client.Listen("unix", remoteSocket)
	if err != nil {
		log.WithError(err).Fatalf("remote listen on %s failed", remoteSocket)
	}
	defer func() {
		err := remoteListener.Close()
		if err != nil {
			log.WithError(err).Warning("Failed to close remote listener")
		}
	}()
	log.Debugf("remote listening %s → local %s", remoteSocket, localSocket)

	for {
		remoteConn, err := remoteListener.Accept()
		if err != nil {
			log.WithError(err).Debug("remote Accept error")
			continue
		}
		go func(rc net.Conn) {
			defer func() {
				err := rc.Close()
				if err != nil {
					log.WithError(err).Warning("failed to close remote connection")
				}
			}()
			// Dial the local unix socket
			lc, err := net.Dial("unix", localSocket)
			if err != nil {
				log.WithError(err).Debugf("local dial %s failed", localSocket)
				return
			}
			defer func() {
				err := lc.Close()
				if err != nil {
					log.WithError(err).Warning("failed to close local connection")
				}
			}()

			done := make(chan struct{}, 2)
			go func() {
				_, err := io.Copy(rc, lc)
				if err != nil {
					log.WithError(err).Warning("failed to copy from remote to local")
				}
				done <- struct{}{}
			}()
			go func() {
				_, err := io.Copy(lc, rc)
				if err != nil {
					log.WithError(err).Warning("failed to copy from local to remote")
				}
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
	defer func() {
		err := term.Restore(stdin, oldState)
		if err != nil {
			log.WithError(err).Warning("failed to restore terminal")
		}
	}()

	sigch := make(chan os.Signal, 1)
	signal.Notify(sigch, os.Interrupt)
	go func() {
		for range sigch {
			err := term.Restore(stdin, oldState)
			if err != nil {
				log.WithError(err).Warning("failed to restore terminal state")
			}
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
