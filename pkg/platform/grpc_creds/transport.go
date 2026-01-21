package grpc_creds

import (
	"context"
	"net"

	log "github.com/sirupsen/logrus"
	"google.golang.org/grpc/credentials"
)

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
		c.log.WithError(err).Warning("getCreds")
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
