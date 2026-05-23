package sshagent

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"sync"

	"golang.org/x/crypto/ssh"
	"golang.org/x/net/http2"
)

const (
	ExtAuthentikAgentTunnel = "agent-tunnel@goauthentik.io"
)

type ExtAuthentikAgentTunnelData struct {
	Data []byte
}

var mtx = sync.Mutex{}

func (atxn *AgentTxn) handleAuthentikAgentTunnel(raw []byte) ([]byte, error) {
	mtx.Lock()
	defer func() {
		atxn.log.Debug("unlock")
		mtx.Unlock()
	}()
	d := ExtAuthentikAgentTunnelData{}
	err := ssh.Unmarshal(raw, &d)
	if err != nil {
		atxn.log.WithError(err).Warning("failed to unmarshal tunnel data")
		return []byte{}, nil
	}

	fmt.Printf("(%d) %+X \n", len(d.Data), d.Data)

	if atxn.tc == nil {
		atxn.log.Debug("new conn")
		c, err := atxn.ag.mls.Dial()
		if err != nil {
			atxn.log.WithError(err).Warning("failed to get new conn")
			return []byte{}, nil
		}
		atxn.tc = c
	}
	n, err := atxn.tc.Write(d.Data)
	atxn.log.Debugf("write %+X (%d, max=%d)", d.Data, n, len(d.Data))
	if err != nil {
		atxn.log.WithError(err).Warning("failed to write")
	}

	var dd = make([]byte, 1024)
	n, err = atxn.tc.Read(dd)
	if err != nil {
		if errors.Is(err, io.EOF) {
			return []byte{}, nil
		}
		atxn.log.WithError(err).Warning("failed to read")
		return []byte{}, err
	}
	atxn.log.Debugf("read %+X (%d), max=%d", dd[:n], n, len(dd))

	fh, err := http2.ReadFrameHeader(bytes.NewBuffer(dd[:n]))
	fmt.Printf("%+v\n", fh)
	fmt.Printf("%+v\n", err)
	return dd[:n], nil
}
