package grpc_creds

import (
	"context"
	"net"

	"github.com/shirou/gopsutil/v4/process"
	"google.golang.org/grpc/credentials"
)

type transportCredentials struct {
}

type AuthInfo struct {
	Creds *Creds
}

func (a AuthInfo) AuthType() string {
	return "socket"
}

func NewTransportCredentials() credentials.TransportCredentials {
	return &transportCredentials{}
}

func (c *transportCredentials) ClientHandshake(ctx context.Context, authority string, conn net.Conn) (net.Conn, credentials.AuthInfo, error) {
	return conn, nil, nil
}

func (c *transportCredentials) ServerHandshake(conn net.Conn) (net.Conn, credentials.AuthInfo, error) {
	var creds *Creds
	var err error
	if unixConn, ok := conn.(*net.UnixConn); ok {
		creds, err = getCreds(unixConn)
		if err != nil {
			return nil, nil, err
		}
		creds.Parent, err = getParent(creds.PID)
		if err != nil {
			return nil, nil, err
		}
		creds.ParentExe, err = creds.Parent.Exe()
		if err != nil {
			return nil, nil, err
		}
	}

	return conn, AuthInfo{
		Creds: creds,
	}, nil
}

func (c *transportCredentials) Info() credentials.ProtocolInfo {
	return credentials.ProtocolInfo{
		SecurityProtocol: "socket",
		SecurityVersion:  "1.0",
	}
}

func (c *transportCredentials) Clone() credentials.TransportCredentials {
	return c
}

func (c *transportCredentials) OverrideServerName(sn string) error {
	return nil
}

type Creds struct {
	Parent    *process.Process
	ParentExe string
	PID       int
	UID       int
	GID       int
}

func GetCreds(conn net.Conn) (*Creds, error) {
	return getCreds(conn)
}

func getParent(pid int) (*process.Process, error) {
	proc, err := process.NewProcess(int32(pid))
	if err != nil {
		return nil, err
	}
	return proc.Parent()
}
