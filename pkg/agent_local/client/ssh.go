package client

import (
	"bytes"
	"context"
	"net"
	"time"

	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
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

	agentClient := agent.NewClient(conn)

	return NewDialer(func(ctx context.Context, s string) (net.Conn, error) {
		return &sshAgentTunnel{
			agent: agentClient,
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

func (sat *sshAgentTunnel) Read(b []byte) (n int, err error) {
	return sat.buff.Read(b)
}

func (sat *sshAgentTunnel) Write(b []byte) (n int, err error) {
	d := ssh.Marshal(sshagent.ExtAuthentikAgentTunnelData{
		Data: b,
	})
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
