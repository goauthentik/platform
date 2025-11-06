//go:build windows

package session

import (
	"errors"
)

func (m *Monitor) terminateSession(session *Session) error {
	return errors.New("Not supported on this platform")
}
