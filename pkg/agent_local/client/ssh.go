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
)

const (
	// ssh-agent(1) provides a UNIX socket at $SSH_AUTH_SOCK.
	sshAuthSock = "SSH_AUTH_SOCK"
)

func NewSSHTunnel(socket string, opts ...opt) (*AgentClient, error) {
	return NewDialer(func(ctx context.Context, s string) (net.Conn, error) {
		return net.Dial("unix", socket)
		// return &sshAgentTunnel{
		// 	agent: agent.NewClient(conn),
		// 	conn:  conn,
		// }, nil
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
	dd, err := retry.DoWithData(
		func() ([]byte, error) {
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
	fmt.Printf("write %d %X\n", len(b), b)
	return len(b), err
}

func (sat *sshAgentTunnel) Write(b []byte) (int, error) {
	d := ssh.Marshal(sshagent.ExtAuthentikAgentTunnelData{
		Data: b,
	})

	fmt.Printf("write %d %X\n", len(b), b)
	r, err := sat.agent.Extension(sshagent.ExtAuthentikAgentTunnel, d)
	if err != nil {
		return 0, err
	}

	_, err = sat.buff.Write(r)
	if err != nil {
		return 0, err
	}
	return len(b), nil
}
