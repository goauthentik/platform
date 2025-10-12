package socket

import "net"

type SocketPermMode int

const (
	SocketOwner SocketPermMode = iota
	SocketEveryone
	SocketAdmin
)

func Listen(name string, perm SocketPermMode) (InfoListener, error) {
	return listen(name, perm)
}

func Connect(name string) (net.Conn, error) {
	return connect(name)
}

type InfoListener interface {
	net.Listener
	Path() string
}

type infoSocket struct {
	net.Listener
	path string
}

func (is infoSocket) Path() string {
	return is.path
}
