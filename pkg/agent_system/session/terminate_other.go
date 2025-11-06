//go:build !linux

package session

import (
	"errors"
)

func (m *Monitor) terminateSession(session *Session) error {
	return errors.New("not supported on this platform")
}
