//go:build !linux

package session

import (
	"errors"

	"goauthentik.io/platform/pkg/pb"
)

func (ss *Server) terminateSession(session *pb.StateSession) error {
	return errors.New("not supported on this platform")
}
