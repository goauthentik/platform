package ssh

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"

	"golang.org/x/crypto/ssh"
	"golang.org/x/term"
)

func (c *SSHClient) Shell(client *ssh.Client) error {
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

	// Request a pseudo terminal
	if term.IsTerminal(int(os.Stdin.Fd())) {
		originalState, err := term.MakeRaw(int(os.Stdin.Fd()))
		if err != nil {
			c.log.WithError(err).Fatal("Failed to set raw mode")
		}
		defer func() {
			err := term.Restore(int(os.Stdin.Fd()), originalState)
			if err != nil {
				c.log.WithError(err).Warn("Failed to restore terminal state")
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
			c.log.Fatalf("Failed to request pty: %v", err)
		}
	}

	// Start shell
	if err := session.Shell(); err != nil {
		c.log.Fatalf("Failed to start shell: %v", err)
	}

	// Wait for session to end
	_ = session.Wait()
	return nil
}

func (c *SSHClient) ReadPassword(prompt string) (string, error) {
	stdin := int(syscall.Stdin)
	oldState, err := term.GetState(stdin)
	if err != nil {
		return "", err
	}
	defer func() {
		err := term.Restore(stdin, oldState)
		if err != nil {
			c.log.WithError(err).Warning("failed to restore terminal")
		}
	}()

	sigch := make(chan os.Signal, 1)
	signal.Notify(sigch, os.Interrupt)
	go func() {
		for range sigch {
			err := term.Restore(stdin, oldState)
			if err != nil {
				c.log.WithError(err).Warning("failed to restore terminal state")
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
