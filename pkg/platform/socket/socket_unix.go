//go:build !windows
// +build !windows

package socket

import (
	"net"
	"os"
)

func listen(path string, perm SocketPermMode) (net.Listener, error) {
	lis, err := net.Listen("unix", path)
	if err != nil {
		return nil, err
	}
	uperm := 0600
	if perm == SocketOwner {
		uperm = 0600
	}
	err = os.Chmod(path, os.FileMode(uperm))
	if err != nil {
		return nil, err
	}
	return lis, nil
}

func connect(path string) (net.Conn, error) {
	return net.Dial("unix", path)
}
