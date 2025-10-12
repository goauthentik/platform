//go:build !(linux || darwin)

package grpc_creds

import (
	"errors"
	"net"
)

func getCreds(conn net.Conn) (*Creds, error) {
	return nil, errors.New("not implemented")
}
