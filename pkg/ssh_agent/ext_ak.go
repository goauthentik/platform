package sshagent

import (
	"io"

	"golang.org/x/crypto/ssh"
)

const (
	ExtAuthentikAgentTunnel = "agent-tunnel@goauthentik.io"
)

type ExtAuthentikAgentTunnelData struct {
	Data []byte
}

func (atxn *AgentTxn) handleAuthentikAgentTunnel(raw []byte) ([]byte, error) {
	d := ExtAuthentikAgentTunnelData{}
	err := ssh.Unmarshal(raw, &d)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to unmarshal tunnel data")
		return []byte{}, nil
	}

	atxn.log.Debugf("%+v\n", raw)
	if atxn.tc == nil {
		atxn.log.Debug("write - new conn")
		atxn.tc, err = atxn.ag.ml.NewConn(d.Data)
		if err != nil {
			atxn.log.WithError(err).Warning("failed to get new conn")
			return []byte{}, nil
		}
	} else {
		atxn.log.Debug("write")
		atxn.tc.clientBuff.Reset()
		atxn.tc.clientBuff.Write(d.Data)
	}

	atxn.log.Debug("read")
	responseBytes, err := io.ReadAll(&atxn.tc.srvBuff)
	if err != nil {
		atxn.log.WithError(err).Warning("Failed to read grpc response")
		return []byte{}, nil
	}
	atxn.log.Debugf("%+v\n", responseBytes)
	return responseBytes, nil
}
