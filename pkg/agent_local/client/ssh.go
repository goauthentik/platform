package client

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"net"
	"time"

	"github.com/avast/retry-go/v4"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
	"golang.org/x/net/http2"
)

const (
	// ssh-agent(1) provides a UNIX socket at $SSH_AUTH_SOCK.
	sshAuthSock = "SSH_AUTH_SOCK"
)

func NewSSHTunnel(socket string, opts ...opt) (*AgentClient, error) {
	conn, err := net.Dial("unix", socket)
	if err != nil {
		return nil, err
	}
	return NewDialer(func(ctx context.Context, s string) (net.Conn, error) {
		return &sshAgentTunnel{
			agent: agent.NewClient(conn),
			conn:  conn,
		}, nil
	}, opts...)
}

type sshAgentTunnel struct {
	agent agent.ExtendedAgent
	conn  net.Conn
	buff  bytes.Buffer
}

func (sat *sshAgentTunnel) Close() error                       { return sat.conn.Close() }
func (sat *sshAgentTunnel) LocalAddr() net.Addr                { return sat.conn.LocalAddr() }
func (sat *sshAgentTunnel) RemoteAddr() net.Addr               { return sat.conn.RemoteAddr() }
func (sat *sshAgentTunnel) SetDeadline(t time.Time) error      { return sat.conn.SetDeadline(t) }
func (sat *sshAgentTunnel) SetReadDeadline(t time.Time) error  { return sat.conn.SetReadDeadline(t) }
func (sat *sshAgentTunnel) SetWriteDeadline(t time.Time) error { return sat.conn.SetWriteDeadline(t) }

var errStall = errors.New("stall")

func (sat *sshAgentTunnel) Read(b []byte) (int, error) {
	fmt.Printf("read %d\n", len(b))
	dd, err := retry.DoWithData(
		func() ([]byte, error) {
			fmt.Printf("read %d\n", len(b))
			if sat.buff.Len() == 0 {
				return []byte{}, errStall
			}
			d := sat.buff.Bytes()
			return d, nil
		},
		retry.Delay(10*time.Microsecond),
		retry.DelayType(retry.BackOffDelay),
		retry.MaxDelay(100*time.Millisecond),
		retry.Attempts(0),
	)
	copy(b, dd)
	return len(dd), err
}

func (sat *sshAgentTunnel) Write(b []byte) (int, error) {
	d := ssh.Marshal(sshagent.ExtAuthentikAgentTunnelData{
		Data: b,
	})
	fmt.Printf("(%d) %+X\n", len(b), b)

	fmt.Printf("'%s'\n", string(b))

	fh, err := http2.ReadFrameHeader(bytes.NewBuffer(b[0:15]))
	fmt.Printf("%+v\n", fh)
	fmt.Printf("%+v\n", err)

	r, err := sat.agent.Extension(sshagent.ExtAuthentikAgentTunnel, d)
	if err != nil {
		return 0, err
	}
	fmt.Printf("write %d %+X\n", len(r), r)
	_, err = sat.buff.Write(r)
	if err != nil {
		return 0, err
	}
	return len(b), nil
}
