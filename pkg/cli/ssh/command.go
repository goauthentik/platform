package ssh

import (
	"os"

	"golang.org/x/crypto/ssh"
)

func (c *SSHClient) command(session *ssh.Session) error {
	// Set up terminal
	session.Stdout = os.Stdout
	session.Stderr = os.Stderr
	session.Stdin = os.Stdin

	return session.Run(c.Command)
}
