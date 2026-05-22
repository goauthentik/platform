package grpc_creds

import (
	"context"
	"fmt"
	"net"
	"strings"

	"google.golang.org/grpc/peer"
)

type Creds struct {
	Proc   *ProcInfo
	Parent *ProcInfo

	PID int
	UID string
	GID string
}

func (c Creds) UniqueProcessID() string {
	firstExe := strings.SplitN(c.Parent.Cmdline, " ", 2)
	return fmt.Sprintf("%s:%s", c.Parent.Exe, firstExe[0])
}

func AuthFromContext(ctx context.Context) *Creds {
	p, ok := peer.FromContext(ctx)
	if !ok {
		return nil
	}
	creds := p.AuthInfo.(AuthInfo).Creds
	return creds
}

func GetCreds(conn net.Conn) (*Creds, error) {
	creds, err := getCreds(conn)
	if err != nil {
		return nil, err
	}
	creds.Proc, err = ProcInfoFrom(int32(creds.PID))
	if err != nil {
		return nil, err
	}
	parent, err := creds.Proc.Parent()
	if err != nil {
		return nil, err
	}
	creds.Parent, err = ProcInfoFrom(parent.Pid)
	if err != nil {
		return nil, err
	}
	return creds, nil
}
