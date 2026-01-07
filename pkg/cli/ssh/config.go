package ssh

import (
	"context"
	"errors"
	"fmt"
	"net"
	"os"
	"path"
	"time"

	progress "github.com/ankddev/conemu-progressbar-go"
	"github.com/skeema/knownhosts"
	"goauthentik.io/platform/pkg/cli/auth/device"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/shared/tui"
	"golang.org/x/crypto/ssh"
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
				fmt.Printf("REMOTE HOST IDENTIFICATION HAS CHANGED for host %s! This may indicate a MitM attack.", hostname)
				return errors.New("hostkey changed")
			} else if knownhosts.IsHostUnknown(err) {
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
				c.log.Infof("Added host %s to known_hosts\n", hostname)
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
