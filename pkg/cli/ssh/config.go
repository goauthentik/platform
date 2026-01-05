package ssh

import (
	"context"
	"errors"
	"fmt"
	"net"
	"os"
	"path"

	"github.com/skeema/knownhosts"
	"goauthentik.io/platform/pkg/cli/auth/device"
	"golang.org/x/crypto/ssh"
)

func (c *SSHClient) getConfig() *ssh.ClientConfig {
	config := &ssh.ClientConfig{
		User: c.user,
		Auth: []ssh.AuthMethod{
			ssh.KeyboardInteractive(func(name, instruction string, questions []string, echos []bool) ([]string, error) {
				c.log.Debugf("name '%s' instruction '%s' questions '%+v' echos '%+v'\n", name, instruction, questions, echos)
				if len(questions) > 0 && questions[0] == "authentik Password: " {
					fmt.Printf("Getting token to access '%s'...\n", c.host)
					cc := device.GetCredentials(c.AgentClient, context.Background(), device.CredentialsOpts{
						Profile:    c.AgentProfile,
						DeviceName: c.host,
					})
					if cc == nil {
						return []string{}, errors.New("failed to exchange token")
					}
					return []string{FormatToken(cc, c.remoteSocketPath)}, nil
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
			}),
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
				if ferr == nil {
					c.log.Infof("Added host %s to known_hosts\n", hostname)
				} else {
					c.log.Infof("Failed to add host %s to known_hosts: %v\n", hostname, ferr)
				}
				return nil // permit previously-unknown hosts (warning: may be insecure)
			}
			return err
		}),
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
