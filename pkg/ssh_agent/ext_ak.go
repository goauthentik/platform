package sshagent

import (
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"golang.org/x/crypto/ssh"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/peer"
)

const (
	ExtAuthentikAgentTunnel       = "agent-tunnel@goauthentik.io"
	SSHAgentExtensionResponseType = 29
)

type ExtAuthentikAgentTunnelData struct {
	Method string
	Data   []byte
}

type ExtAuthentikAgentTunnelDataResp struct {
	Typ  uint8
	Ext  string
	Data []byte
}

func (atxn *AgentTxn) handleAuthentikAgentTunnel(raw []byte) ([]byte, error) {
	d := ExtAuthentikAgentTunnelData{}
	err := ssh.Unmarshal(raw, &d)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to unmarshal tunnel data")
		return []byte{}, nil
	}

	creds, err := grpc_creds.GetCreds(atxn.conn)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to get caller creds")
		return []byte{}, nil
	}
	rr, err := atxn.ag.grpc.CallWithPeer(atxn.ctx, d.Method, d.Data, &peer.Peer{
		AuthInfo: grpc_creds.AuthInfo{
			CommonAuthInfo: credentials.CommonAuthInfo{
				SecurityLevel: credentials.PrivacyAndIntegrity,
			},
			Creds: creds,
		},
	})
	if err != nil {
		atxn.log.WithError(err).Warning("failed to call method")
		return []byte{}, nil
	}

	rd := ExtAuthentikAgentTunnelDataResp{
		Typ: SSHAgentExtensionResponseType,
		Ext: ExtAuthentikAgentTunnel,
		Data: ssh.Marshal(ExtAuthentikAgentTunnelData{
			Method: d.Method,
			Data:   rr,
		}),
	}
	return ssh.Marshal(rd), nil
}
