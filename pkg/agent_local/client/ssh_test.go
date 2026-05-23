package client

import (
	"errors"
	"net"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
)

// mockExtendedAgent implements agent.ExtendedAgent with a configurable Extension callback.
type mockExtendedAgent struct {
	extensionFunc func(extensionType string, contents []byte) ([]byte, error)
}

func (m *mockExtendedAgent) List() ([]*agent.Key, error)                               { return nil, nil }
func (m *mockExtendedAgent) Sign(key ssh.PublicKey, data []byte) (*ssh.Signature, error) {
	return nil, nil
}
func (m *mockExtendedAgent) Add(key agent.AddedKey) error         { return nil }
func (m *mockExtendedAgent) Remove(key ssh.PublicKey) error       { return nil }
func (m *mockExtendedAgent) RemoveAll() error                     { return nil }
func (m *mockExtendedAgent) Lock(passphrase []byte) error         { return nil }
func (m *mockExtendedAgent) Unlock(passphrase []byte) error       { return nil }
func (m *mockExtendedAgent) Signers() ([]ssh.Signer, error) { return nil, nil }
func (m *mockExtendedAgent) SignWithFlags(key ssh.PublicKey, data []byte, flags agent.SignatureFlags) (*ssh.Signature, error) {
	return nil, nil
}
func (m *mockExtendedAgent) Extension(extensionType string, contents []byte) ([]byte, error) {
	return m.extensionFunc(extensionType, contents)
}

func newTestTunnel(ma *mockExtendedAgent) *sshAgentTunnel {
	c1, _ := net.Pipe()
	return &sshAgentTunnel{agent: ma, conn: c1}
}

func Test_SSHAgentTunnel_Write(t *testing.T) {
	var capturedType string
	var capturedContents []byte
	response := []byte("response-data")

	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			capturedType = extensionType
			capturedContents = contents
			return response, nil
		},
	}
	sat := newTestTunnel(ma)
	n, err := sat.Write([]byte("hello"))
	assert.NoError(t, err)
	assert.Equal(t, 5, n)
	assert.Equal(t, sshagent.ExtAuthentikAgentTunnel, capturedType)

	// Verify the payload is SSH-marshaled ExtAuthentikAgentTunnelData.
	var parsed sshagent.ExtAuthentikAgentTunnelData
	ssh.Unmarshal(capturedContents, &parsed)
	assert.Equal(t, []byte("hello"), parsed.Data)

	// The extension response should be buffered for subsequent reads.
	assert.Equal(t, response, sat.buff.Bytes())
}

func Test_SSHAgentTunnel_Write_Error(t *testing.T) {
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			return nil, errors.New("extension failed")
		},
	}
	sat := newTestTunnel(ma)
	n, err := sat.Write([]byte("data"))
	assert.Error(t, err)
	assert.Equal(t, 0, n)
	assert.Empty(t, sat.buff.Bytes())
}

func Test_SSHAgentTunnel_Read_FromBuffer(t *testing.T) {
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			return nil, nil
		},
	}
	sat := newTestTunnel(ma)
	expected := []byte("buffered-data")
	sat.buff.Write(expected)

	buf := make([]byte, 64)
	n, err := sat.Read(buf)
	assert.NoError(t, err)
	assert.Equal(t, len(expected), n)
	assert.Equal(t, expected, buf[:n])
}

func Test_SSHAgentTunnel_StubMethods(t *testing.T) {
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			return nil, nil
		},
	}
	c1, c2 := net.Pipe()
	defer c2.Close()
	sat := &sshAgentTunnel{agent: ma, conn: c1}

	assert.NotNil(t, sat.LocalAddr())
	assert.NotNil(t, sat.RemoteAddr())
	assert.NoError(t, sat.SetDeadline(time.Time{}))
	assert.NoError(t, sat.SetReadDeadline(time.Time{}))
	assert.NoError(t, sat.SetWriteDeadline(time.Time{}))
	assert.NoError(t, sat.Close())
}
