//go:build !(linux || darwin)

package grpc_creds

import (
	"errors"
	"net"
)

func getCreds(conn *net.UnixConn) (*Creds, error) {
	return nil, errors.New("not implemented")
}
