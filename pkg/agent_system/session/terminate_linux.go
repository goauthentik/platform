//go:build linux

package session

import (
	"os"
	"syscall"
	"time"
)

func (m *Monitor) terminateSession(session *Session) error {
	_ = os.Remove(session.LocalSocket)

	// Try graceful termination first
	if err := syscall.Kill(int(session.PID), syscall.SIGTERM); err != nil {
		return err
	}

	// Wait a bit, then force kill if needed
	time.Sleep(5 * time.Second)

	// Check if process still exists
	if err := syscall.Kill(int(session.PID), 0); err == nil {
		// Process still exists, force kill
		return syscall.Kill(int(session.PID), syscall.SIGKILL)
	}

	return nil
}
