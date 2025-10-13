package socket

import (
	"net"

	"goauthentik.io/cli/pkg/platform/pstr"
)

type SocketPermMode int

const (
	SocketOwner SocketPermMode = iota
	SocketEveryone
	SocketAdmin
)

func Listen(name pstr.PlatformString, perm SocketPermMode) (InfoListener, error) {
	return listen(name, perm)
}

func Connect(name pstr.PlatformString) (net.Conn, error) {
	return connect(name)
}

type InfoListener interface {
	net.Listener
	Path() pstr.PlatformString
}

type infoSocket struct {
	net.Listener
	path pstr.PlatformString
}

func (is infoSocket) Path() pstr.PlatformString {
	return is.path
}
