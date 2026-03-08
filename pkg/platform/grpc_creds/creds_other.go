//go:build !(linux || darwin || windows)

package grpc_creds

import (
	"errors"
	"net"
)

func getCreds(conn net.Conn) (*Creds, error) {
	return nil, errors.ErrUnsupported
}
