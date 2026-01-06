package ssh

import (
	"io"
	"os"

	"github.com/pkg/errors"
	"golang.org/x/crypto/ssh"
)

func (c *SSHClient) command(client *ssh.Client) error {
	// Create a session for interactive shell
	session, err := client.NewSession()
	if err != nil {
		c.log.WithError(err).Fatal("Failed to create session")
	}
	defer func() {
		err := session.Close()
		if err != nil && !errors.Is(err, io.EOF) {
			c.log.WithError(err).Warning("Failed to close session")
		}
	}()

	// Set up terminal
	session.Stdout = os.Stdout
	session.Stderr = os.Stderr
	session.Stdin = os.Stdin

	return session.Run(c.Command)
}
