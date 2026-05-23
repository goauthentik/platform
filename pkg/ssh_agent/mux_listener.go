package sshagent

import (
	"net"
)

var _ net.Listener = &MuxListener{}

type MuxListener struct {
	conn chan net.Conn
	done chan struct{}
}

func (ml *MuxListener) AddConn(nc net.Conn) {
	ml.conn <- nc
}

func (ml *MuxListener) Accept() (net.Conn, error) {
	select {
	case <-ml.done:
		return nil, nil
	case conn := <-ml.conn:
		return conn, nil
	}
}

func (ml *MuxListener) Addr() net.Addr {
	return memAddr{}
}

func (ml *MuxListener) Close() error {
	ml.done <- struct{}{}
	return nil
}

type memAddr struct{}

func (memAddr) Network() string { return "mem" }
func (memAddr) String() string  { return "mem" }
