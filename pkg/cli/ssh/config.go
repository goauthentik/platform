package ssh

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"net"
	"os"
	"path"
	"strings"
	"time"

	progress "github.com/ankddev/conemu-progressbar-go"
	"github.com/skeema/knownhosts"
	"goauthentik.io/platform/pkg/cli/auth/device"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/shared/tui"
	"golang.org/x/crypto/ssh"
	"golang.org/x/term"
)

const PAMPrompt = "authentik Password: "

func (c *SSHClient) getTokenIfNeeded() (string, error) {
	progress.SetIndeterminateProgress()
	defer progress.ClearProgress()
	fmt.Println(tui.InlineStyle().Render(fmt.Sprintf("authentik: Getting token to access '%s'...", c.host)))
	cc := device.GetCredentials(c.AgentClient, context.Background(), device.CredentialsOpts{
		Profile:    c.AgentProfile,
		DeviceName: c.host,
	})
	if cc == nil {
		return "", errors.New("failed to exchange token")
	}
	ft := FormatToken(cc, c.remoteSocketPath)
	return ft, nil
}

func (c *SSHClient) auth(name, instruction string, questions []string, echos []bool) ([]string, error) {
	c.log.Debugf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
	if len(questions) > 0 && questions[0] == PAMPrompt {
		if c.agentToken == "" {
			token, err := c.getTokenIfNeeded()
			if err != nil {
				return []string{}, err
			}
			c.agentToken = token
		}
		return []string{c.agentToken}, nil
	}
	ans := []string{}
	for _, q := range questions {
		l, err := c.ReadPassword(q)
		fmt.Println("")
		if err != nil {
			return ans, err
		}
		ans = append(ans, l)
	}
	return ans, nil
}

func (c *SSHClient) getConfig() *ssh.ClientConfig {
	config := &ssh.ClientConfig{
		User: c.user,
		Auth: []ssh.AuthMethod{
			ssh.KeyboardInteractive(c.auth),
		},
		HostKeyCallback: ssh.HostKeyCallback(func(hostname string, remote net.Addr, key ssh.PublicKey) error {
			innerCallback := c.knownHosts.HostKeyCallback()
			err := innerCallback(hostname, remote, key)
			if knownhosts.IsHostKeyChanged(err) {
				fmt.Printf("REMOTE HOST IDENTIFICATION HAS CHANGED for host %s! This may indicate a MitM attack.\n", hostname)
				return errors.New("hostkey changed")
			} else if knownhosts.IsHostUnknown(err) {
				if !term.IsTerminal(int(os.Stdin.Fd())) {
					return errors.New("host key verification failed: unknown host and stdin is not a terminal")
				}
				fmt.Printf("The authenticity of host '%s (%s)' can't be established.\n", hostname, remote.String())
				fmt.Printf("%s key fingerprint is %s.\n", key.Type(), ssh.FingerprintSHA256(key))
				fmt.Print("Are you sure you want to continue connecting (yes/no)? ")
				answer, rerr := bufio.NewReader(os.Stdin).ReadString('\n')
				if rerr != nil {
					return fmt.Errorf("failed to read user input: %w", rerr)
				}
				if strings.TrimSpace(strings.ToLower(answer)) != "yes" {
					return errors.New("host key verification rejected by user")
				}
				f, ferr := os.OpenFile(c.knownHostsFile, os.O_APPEND|os.O_WRONLY, 0600)
				if ferr == nil {
					defer func() {
						err := f.Close()
						if err != nil {
							c.log.WithError(err).Warning("failed to close known_hosts file")
						}
					}()
					ferr = knownhosts.WriteKnownHost(f, hostname, remote, key)
				}
				if ferr != nil {
					c.log.Infof("Failed to add host %s to known_hosts: %v\n", hostname, ferr)
					return ferr
				}
				fmt.Printf("Warning: Permanently added '%s' (%s) to the list of known hosts.\n", hostname, key.Type())
				c.log.Infof("Added host %s to known_hosts\n", hostname)
				return nil
			}
			return err
		}),
		HostKeyAlgorithms: c.knownHosts.HostKeyAlgorithms(c.host),
		ClientVersion:     fmt.Sprintf("SSH-2.0-authentik-cli/%s", meta.FullVersion()),
		Timeout:           5 * time.Second,
	}
	if c.Insecure {
		config.HostKeyCallback = ssh.InsecureIgnoreHostKey()
	}
	return config
}

func DefaultKnownHostsPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return path.Join(home, ".ssh/known_hosts"), err
}
