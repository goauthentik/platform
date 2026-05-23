package sshagent

import (
	"context"
	"crypto/rand"
	"net"
	"sync"

	log "github.com/sirupsen/logrus"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
)

type AgentTxn struct {
	ag   *Agent
	log  *log.Entry
	conn net.Conn

	hostKey ssh.PublicKey

	crt *ssh.Certificate
	cpk ssh.Signer

	ctx context.Context

	tunnelConn net.Conn
	tunnelMtx  sync.Mutex
}

func init() {
	var _ agent.Agent = &AgentTxn{}
	var _ agent.ExtendedAgent = &AgentTxn{}
}

// No-op since we use SignWithFlags()
func (atxn *AgentTxn) Sign(key ssh.PublicKey, data []byte) (*ssh.Signature, error) { return nil, nil }

// Stub methods for things we don't implement
func (atxn *AgentTxn) Add(key agent.AddedKey) error   { atxn.log.Debug("Add()"); return nil }
func (atxn *AgentTxn) Remove(key ssh.PublicKey) error { atxn.log.Debug("Remove()"); return nil }
func (atxn *AgentTxn) RemoveAll() error               { atxn.log.Debug("RemoveAll()"); return nil }
func (atxn *AgentTxn) Lock(passphrase []byte) error   { atxn.log.Debug("Lock()"); return nil }
func (atxn *AgentTxn) Unlock(passphrase []byte) error { atxn.log.Debug("Unlock()"); return nil }
func (atxn *AgentTxn) Signers() ([]ssh.Signer, error) {
	atxn.log.Debug("Signers()")
	return []ssh.Signer{}, nil
}

func (atxn *AgentTxn) List() ([]*agent.Key, error) {
	atxn.log.Debug("List()")
	return []*agent.Key{
		{
			Format:  atxn.crt.Type(),
			Blob:    atxn.crt.Marshal(),
			Comment: "",
		},
	}, nil
}

func (atxn *AgentTxn) SignWithFlags(key ssh.PublicKey, data []byte, flags agent.SignatureFlags) (*ssh.Signature, error) {
	atxn.log.Debugf("SignWithFlags(%s, %v)", key.Type(), flags)

	return atxn.cpk.Sign(rand.Reader, data)
}

func (atxn *AgentTxn) Extension(extensionType string, contents []byte) ([]byte, error) {
	atxn.log.Debugf("Extension(%s, %d)", extensionType, len(contents))
	switch extensionType {
	case ExtOpenSSHSessionBind:
		sb, err := ParseSessionBind(contents)
		if err != nil {
			return []byte{}, err
		}
		atxn.hostKey = sb.HostKey
		crt, sign, err := atxn.generateKey()
		if err != nil {
			atxn.log.WithError(err).Warning("failed to generate cert")
			return nil, err
		}
		atxn.crt = crt
		atxn.cpk = sign
	case ExtAuthentikAgentTunnel:
		return atxn.handleAuthentikAgentTunnel(contents)
	}
	return []byte{}, nil
}

func (atxn *AgentTxn) Close() error {
	if atxn.tunnelConn != nil {
		return atxn.tunnelConn.Close()
	}
	return nil
}
