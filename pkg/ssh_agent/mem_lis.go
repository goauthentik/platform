package sshagent

import (
	"bytes"
	"errors"
	"io"
	"net"
	"sync"
	"time"
)

type MemListener struct {
	connCh chan net.Conn
	closed chan struct{}
	once   sync.Once
}

func NewMemoryListener() *MemListener {
	return &MemListener{
		connCh: make(chan net.Conn, 16),
		closed: make(chan struct{}),
	}
}

// Accept blocks until a new connection is injected or the listener is closed.
// Called internally by grpc.Server — you don't call this directly.
func (l *MemListener) Accept() (net.Conn, error) {
	select {
	case conn := <-l.connCh:
		return conn, nil
	case <-l.closed:
		return nil, io.ErrClosedPipe
	}
}

func (l *MemListener) Close() error {
	l.once.Do(func() { close(l.closed) })
	return nil
}

func (l *MemListener) Addr() net.Addr {
	return memAddr{}
}

func (l *MemListener) NewConn(data []byte) (*memConn, error) {
	select {
	case <-l.closed:
		return nil, io.ErrClosedPipe
	default:
	}

	mc := &memConn{
		srvBuff:    bytes.Buffer{},
		clientBuff: bytes.Buffer{},
	}
	_, _ = mc.clientBuff.Write(data)

	select {
	case l.connCh <- mc:
	case <-l.closed:
		return nil, io.ErrClosedPipe
	}

	return mc, nil
}

type memConn struct {
	srvBuff    bytes.Buffer
	clientBuff bytes.Buffer
}

func (mc *memConn) Close() error                       { return nil }
func (mc *memConn) LocalAddr() net.Addr                { return memAddr{} }
func (mc *memConn) RemoteAddr() net.Addr               { return memAddr{} }
func (mc *memConn) SetDeadline(t time.Time) error      { return nil }
func (mc *memConn) SetReadDeadline(t time.Time) error  { return nil }
func (mc *memConn) SetWriteDeadline(t time.Time) error { return nil }

func (mc *memConn) Read(b []byte) (int, error) {
	n, err := mc.clientBuff.Read(b)
	if errors.Is(err, io.EOF) {
		return n, nil
	}
	return n, err
}
func (mc *memConn) Write(b []byte) (int, error) {
	return mc.srvBuff.Write(b)
}

type memAddr struct{}

func (memAddr) Network() string { return "mem" }
func (memAddr) String() string  { return "mem" }
