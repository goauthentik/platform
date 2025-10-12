package socket

import "net"

type SocketPermMode int

const (
	SocketOwner SocketPermMode = iota
	SocketEveryone
	SocketAdmin
)

func Listen(name string, perm SocketPermMode) (net.Listener, error) {
	return listen(name, perm)
}

func Connect(name string) (net.Conn, error) {
	return connect(name)
}
