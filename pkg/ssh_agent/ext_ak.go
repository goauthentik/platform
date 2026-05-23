package sshagent

import (
	"errors"
	"io"
	"time"

	"golang.org/x/crypto/ssh"
)

const (
	ExtAuthentikAgentTunnel = "agent-tunnel@goauthentik.io"
)

type ExtAuthentikAgentTunnelData struct {
	Data []byte
}

func (atxn *AgentTxn) handleAuthentikAgentTunnel(raw []byte) ([]byte, error) {
	atxn.log.Debug("lock")
	atxn.tunnelMtx.Lock()
	defer func() {
		atxn.log.Debug("unlock")
		atxn.tunnelMtx.Unlock()
	}()
	d := ExtAuthentikAgentTunnelData{}
	err := ssh.Unmarshal(raw, &d)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to unmarshal tunnel data")
		return []byte{}, nil
	}

	if atxn.tunnelConn == nil {
		atxn.log.Debug("new conn")
		c, err := atxn.ag.mls.Dial()
		if err != nil {
			atxn.log.WithError(err).Warning("failed to get new conn")
			return []byte{}, nil
		}
		atxn.tunnelConn = c
	}
	n, err := atxn.tunnelConn.Write(d.Data)
	atxn.log.Debugf("write %+X (%d, max=%d)", d.Data, n, len(d.Data))
	if err != nil {
		atxn.log.WithError(err).Warning("failed to write")
	}

	var dd = make([]byte, 1024)
	atxn.tunnelConn.SetDeadline(time.Now().Add(2 * time.Second))
	n, err = atxn.tunnelConn.Read(dd)
	if err != nil {
		// bufcon's timeout error doesn't correctly use os.ErrDeadlineExceeded
		if err.Error() == "i/o timeout" {
			atxn.log.Debug("deadline")
			return []byte{}, nil
		}
		if errors.Is(err, io.EOF) {
			atxn.log.Debug("eof")
			return []byte{}, nil
		}
		atxn.log.WithError(err).Warning("failed to read")
		return []byte{}, err
	}
	atxn.log.Debugf("read %+X (%d), max=%d", dd[:n], n, len(dd))
	return dd[:n], nil
}
