package sshagent

import (
	"golang.org/x/crypto/ssh"
)

const (
	ExtAuthentikAgentTunnel = "agent-tunnel@goauthentik.io"
)

type ExtAuthentikAgentTunnelData struct {
	Method string
	Data   []byte
}

func (atxn *AgentTxn) handleAuthentikAgentTunnel(raw []byte) ([]byte, error) {
	d := ExtAuthentikAgentTunnelData{}
	err := ssh.Unmarshal(raw, &d)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to unmarshal tunnel data")
		return []byte{}, nil
	}

	rr, err := atxn.ag.grpc.Call(atxn.ctx, d.Method, d.Data)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to call method")
		return []byte{}, nil
	}

	rd := ExtAuthentikAgentTunnelData{
		Method: d.Method,
		Data:   rr,
	}
	return ssh.Marshal(rd), nil
}
