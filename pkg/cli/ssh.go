package cli

import (
	"encoding/base64"
	"errors"
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
	"github.com/skeema/knownhosts"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/cli/auth/device"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/socket"
	"golang.org/x/crypto/ssh"
	"golang.org/x/term"
	"google.golang.org/protobuf/proto"
)

var insecure = false

var sshCmd = &cobra.Command{
	Use:          "ssh",
	Short:        "Establish an SSH connection with `host`.",
	Args:         cobra.MinimumNArgs(1),
	SilenceUsage: true,
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

		uid := uuid.New().String()
		remoteSocketPath := fmt.Sprintf("/var/run/authentik/agent-%s.sock", uid)

		khf, err := DefaultKnownHostsPath()
		if err != nil {
			log.WithError(err).Warning("failed to locate known_hosts")
			return err
		}
		if _, err := os.Stat(khf); os.IsNotExist(err) {
			_, err := os.OpenFile(khf, os.O_CREATE, 0600)
			if err != nil {
				log.WithError(err).Warning("failed to create known_hosts file")
				return err
			}
		}
		kh, err := knownhosts.NewDB(khf)
		if err != nil {
			log.Fatal("Failed to read known_hosts: ", err)
		}

		config := &ssh.ClientConfig{
			User: user,
			Auth: []ssh.AuthMethod{
				ssh.KeyboardInteractive(func(name, instruction string, questions []string, echos []bool) ([]string, error) {
					log.Debugf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
					if len(questions) > 0 && questions[0] == "authentik Password: " {
						fmt.Printf("Getting token to access '%s'...\n", host)
						cc := device.GetCredentials(c, cmd.Context(), device.CredentialsOpts{
							Profile:    profile,
							DeviceName: host,
						})
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
			HostKeyCallback: ssh.HostKeyCallback(func(hostname string, remote net.Addr, key ssh.PublicKey) error {
				innerCallback := kh.HostKeyCallback()
				err := innerCallback(hostname, remote, key)
				if knownhosts.IsHostKeyChanged(err) {
					fmt.Printf("REMOTE HOST IDENTIFICATION HAS CHANGED for host %s! This may indicate a MitM attack.", hostname)
					return errors.New("hostkey changed")
				} else if knownhosts.IsHostUnknown(err) {
					f, ferr := os.OpenFile(khf, os.O_APPEND|os.O_WRONLY, 0600)
					if ferr == nil {
						defer func() {
							err := f.Close()
							if err != nil {
								log.WithError(err).Warning("failed to close known_hosts file")
							}
						}()
						ferr = knownhosts.WriteKnownHost(f, hostname, remote, key)
					}
					if ferr == nil {
						log.Infof("Added host %s to known_hosts\n", hostname)
					} else {
						log.Infof("Failed to add host %s to known_hosts: %v\n", hostname, ferr)
					}
					return nil // permit previously-unknown hosts (warning: may be insecure)
				}
				return err
			}),
		}
		if insecure {
			config.HostKeyCallback = ssh.InsecureIgnoreHostKey()
		}
		client, err := ssh.Dial("tcp", net.JoinHostPort(host, port), config)
		if err != nil {
			return err
		}
		defer func() {
			err := client.Close()
			if err != nil && !errors.Is(err, net.ErrClosed) {
				log.WithError(err).Warning("Failed to close client")
			}
		}()

		go func() {
			err := ForwardAgentSocket(remoteSocketPath, client)
			if err != nil {
				fmt.Printf("Warning: %v\n", err.Error())
			}
		}()
		return Shell(client)
	},
}

func init() {
	sshCmd.Flags().BoolVarP(&insecure, "insecure", "i", false, "Insecure host-key checking, use with caution!")
	rootCmd.AddCommand(sshCmd)
}

func FormatToken(cc *device.DeviceCredentialOutput, rtp string) string {
	msg := pb.SSHTokenAuthentication{
		Token:       cc.AccessToken,
		LocalSocket: rtp,
	}
	rv, err := proto.Marshal(&msg)
	if err != nil {
		panic(err)
	}
	return fmt.Sprintf("\u200b%s", base64.StdEncoding.EncodeToString(rv))
}

func ForwardAgentSocket(remoteSocket string, client *ssh.Client) error {
	localSocket := types.GetAgentSocketPath()
	remoteListener, err := client.Listen("unix", remoteSocket)
	if err != nil {
		return err
	}
	defer func() {
		err := remoteListener.Close()
		if err != nil {
			log.WithError(err).Warning("Failed to close remote listener")
		}
	}()
	log.Debugf("remote listening %s → local %s", remoteSocket, localSocket.ForCurrent())

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
			lc, err := socket.Connect(localSocket)
			if err != nil {
				log.WithError(err).Debugf("local dial %s failed", localSocket.ForCurrent())
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
				if err != nil && !strings.Contains(err.Error(), "use of closed network connection") {
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

func Shell(client *ssh.Client) error {
	// Create a session for interactive shell
	session, err := client.NewSession()
	if err != nil {
		log.WithError(err).Fatal("Failed to create session")
	}
	defer func() {
		err := session.Close()
		if err != nil && !errors.Is(err, io.EOF) {
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
		term := os.Getenv("TERM")
		if term == "" {
			term = "xterm"
		}
		if err := session.RequestPty(term, height, width, ssh.TerminalModes{}); err != nil {
			log.Fatalf("Failed to request pty: %v", err)
		}
	}

	// Start shell
	if err := session.Shell(); err != nil {
		log.Fatalf("Failed to start shell: %v", err)
	}

	// Wait for session to end
	_ = session.Wait()
	return nil
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
