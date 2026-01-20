package grpc_creds

import (
	"context"
	"fmt"
	"net"
	"strings"

	"github.com/shirou/gopsutil/v4/process"
	log "github.com/sirupsen/logrus"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/peer"
)

func AuthFromContext(ctx context.Context) *Creds {
	p, ok := peer.FromContext(ctx)
	if !ok {
		return nil
	}
	creds := p.AuthInfo.(AuthInfo).Creds
	return creds
}

type AuthInfo struct {
	credentials.CommonAuthInfo
	Creds *Creds
}

func (a AuthInfo) AuthType() string {
	return "socket"
}

type transportCredentials struct {
	log *log.Entry
}

func NewTransportCredentials(logger *log.Entry) credentials.TransportCredentials {
	return &transportCredentials{
		log: logger,
	}
}

func (c *transportCredentials) ClientHandshake(ctx context.Context, authority string, conn net.Conn) (net.Conn, credentials.AuthInfo, error) {
	return conn, nil, nil
}

func (c *transportCredentials) ServerHandshake(conn net.Conn) (net.Conn, credentials.AuthInfo, error) {
	creds, err := getCreds(conn)
	if err != nil {
		return nil, nil, err
	}
	creds.Proc, err = ProcInfoFrom(int32(creds.PID))
	if err != nil {
		return nil, nil, err
	}
	parent, err := creds.Proc.Parent()
	if err != nil {
		return nil, nil, err
	}
	creds.Parent, err = ProcInfoFrom(parent.Pid)
	if err != nil {
		return nil, nil, err
	}
	return conn, AuthInfo{
		CommonAuthInfo: credentials.CommonAuthInfo{
			SecurityLevel: credentials.PrivacyAndIntegrity,
		},
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

type ProcInfo struct {
	*process.Process
	Exe     string
	Cmdline string
}

func ProcInfoFrom(pid int32) (*ProcInfo, error) {
	p, err := process.NewProcess(pid)
	if err != nil {
		return nil, err
	}
	pi := &ProcInfo{Process: p}
	pi.Exe, err = p.Exe()
	if err != nil {
		return pi, err
	}
	pi.Cmdline, err = p.Cmdline()
	if err != nil {
		return pi, err
	}
	return pi, nil
}

func (pi *ProcInfo) String() string {
	return fmt.Sprintf("Process <id=%d, exe=%s, cmdline=%s>", pi.Pid, pi.Exe, pi.Cmdline)
}

type Creds struct {
	Proc   *ProcInfo
	Parent *ProcInfo

	PID int
	UID int
	GID int
}

func (c Creds) UniqueProcessID() string {
	firstExe := strings.SplitN(c.Parent.Cmdline, " ", 2)
	return fmt.Sprintf("%s:%s", c.Parent.Exe, firstExe[0])
}
