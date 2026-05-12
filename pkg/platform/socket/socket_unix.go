//go:build !windows

package socket

import (
	"net"
	"os"
	"path"
	"syscall"

	"goauthentik.io/platform/pkg/platform/pstr"
)

func listen(spath pstr.PlatformString, perm SocketPermMode) (InfoListener, error) {
	p := spath.ForCurrent()
	uperm := 0700
	switch perm {
	case SocketOwner:
		uperm = 0600
	case SocketEveryone:
		uperm = 0666
	}
	err := os.MkdirAll(path.Dir(p), os.FileMode(uperm|0100))
	if err != nil {
		return nil, err
	}
	_ = os.Remove(p)
	// Restrict umask before socket creation to eliminate the TOCTOU window where
	// the socket would briefly exist with permissive default permissions.
	oldUmask := syscall.Umask(0777 &^ uperm)
	lis, err := net.Listen("unix", p)
	syscall.Umask(oldUmask)
	if err != nil {
		return nil, err
	}
	err = os.Chmod(p, os.FileMode(uperm))
	if err != nil {
		return nil, err
	}
	return infoSocket{lis, spath}, nil
}

func connect(path pstr.PlatformString) (net.Conn, error) {
	return net.Dial("unix", path.ForCurrent())
}
