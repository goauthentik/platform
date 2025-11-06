//go:build !linux

package session

import (
	"errors"

	"goauthentik.io/platform/pkg/pb"
)

func (m *Monitor) terminateSession(session *pb.StateSession) error {
	return errors.New("not supported on this platform")
}
