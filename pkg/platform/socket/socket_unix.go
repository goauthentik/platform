//go:build !windows
// +build !windows

package socket

import (
	"net"
	"os"
	"path"
)

func listen(spath string, perm SocketPermMode) (InfoListener, error) {
	uperm := 0700
	switch perm {
	case SocketOwner:
		uperm = 0600
	case SocketEveryone:
		uperm = 0666
	}
	err := os.MkdirAll(path.Dir(spath), os.FileMode(uperm))
	if err != nil {
		return nil, err
	}
	err = os.Remove(spath)
	if err != nil {
		return nil, err
	}
	lis, err := net.Listen("unix", spath)
	if err != nil {
		return nil, err
	}
	err = os.Chmod(spath, os.FileMode(uperm))
	if err != nil {
		return nil, err
	}
	return infoSocket{lis, spath}, nil
}

func connect(path string) (net.Conn, error) {
	return net.Dial("unix", path)
}
