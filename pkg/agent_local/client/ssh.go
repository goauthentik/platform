package client

import (
	"context"
	"net"

	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
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
	opts = append(opts, sshTunnelOpt(conn))
	return NewDialer(func(ctx context.Context, s string) (net.Conn, error) {
		return net.Dial("unix", "/dev/null")
	}, opts...)
}

func sshTunnelOpt(c net.Conn) opt {
	return sshAgentOpt(agent.NewClient(c))
}

func sshAgentOpt(ag agent.ExtendedAgent) opt {
	return func(ac *AgentClient) (grpc.UnaryClientInterceptor, grpc.StreamClientInterceptor) {
		return func(ctx context.Context, method string, req, reply any, cc *grpc.ClientConn, invoker grpc.UnaryInvoker, opts ...grpc.CallOption) error {
			d, err := proto.Marshal(req.(proto.Message))
			if err != nil {
				return err
			}
			data := sshagent.ExtAuthentikAgentTunnelData{
				Method: method,
				Data:   d,
			}
			res, err := ag.Extension(sshagent.ExtAuthentikAgentTunnel, ssh.Marshal(data))
			if err != nil {
				return err
			}
			rd := sshagent.ExtAuthentikAgentTunnelData{}
			if err := ssh.Unmarshal(res, &rd); err != nil {
				return err
			}
			proto.Unmarshal(rd.Data, reply.(proto.Message))
			return nil
		}, nil
	}
}
