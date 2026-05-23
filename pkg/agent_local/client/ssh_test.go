package client

import (
	"context"
	"errors"
	"testing"

	"github.com/stretchr/testify/assert"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/emptypb"
)

// mockExtendedAgent implements agent.ExtendedAgent with a configurable Extension callback.
type mockExtendedAgent struct {
	extensionFunc func(extensionType string, contents []byte) ([]byte, error)
}

func (m *mockExtendedAgent) List() ([]*agent.Key, error) { return nil, nil }
func (m *mockExtendedAgent) Sign(key ssh.PublicKey, data []byte) (*ssh.Signature, error) {
	return nil, nil
}
func (m *mockExtendedAgent) Add(key agent.AddedKey) error   { return nil }
func (m *mockExtendedAgent) Remove(key ssh.PublicKey) error { return nil }
func (m *mockExtendedAgent) RemoveAll() error               { return nil }
func (m *mockExtendedAgent) Lock(passphrase []byte) error   { return nil }
func (m *mockExtendedAgent) Unlock(passphrase []byte) error { return nil }
func (m *mockExtendedAgent) Signers() ([]ssh.Signer, error) { return nil, nil }
func (m *mockExtendedAgent) SignWithFlags(key ssh.PublicKey, data []byte, flags agent.SignatureFlags) (*ssh.Signature, error) {
	return nil, nil
}
func (m *mockExtendedAgent) Extension(extensionType string, contents []byte) ([]byte, error) {
	return m.extensionFunc(extensionType, contents)
}

func interceptorFromAgent(ma agent.ExtendedAgent) grpc.UnaryClientInterceptor {
	unary, _ := sshAgentOpt(ma)(nil)
	return unary
}

func Test_sshTunnelOpt_Success(t *testing.T) {
	const method = "/test.Service/Method"

	replyData, _ := proto.Marshal(&emptypb.Empty{})
	response := ssh.Marshal(sshagent.ExtAuthentikAgentTunnelData{
		Method: method,
		Data:   replyData,
	})

	var capturedType string
	var capturedContents []byte
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			capturedType = extensionType
			capturedContents = contents
			return response, nil
		},
	}

	interceptor := interceptorFromAgent(ma)
	req := &emptypb.Empty{}
	reply := &emptypb.Empty{}
	assert.NoError(t, interceptor(context.Background(), method, req, reply, nil, nil))

	assert.Equal(t, sshagent.ExtAuthentikAgentTunnel, capturedType)

	var parsed sshagent.ExtAuthentikAgentTunnelData
	assert.NoError(t, ssh.Unmarshal(capturedContents, &parsed))
	assert.Equal(t, method, parsed.Method)
	reqData, _ := proto.Marshal(req)
	assert.Equal(t, reqData, parsed.Data)
}

func Test_sshTunnelOpt_ExtensionError(t *testing.T) {
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			return nil, errors.New("extension failed")
		},
	}
	interceptor := interceptorFromAgent(ma)
	err := interceptor(context.Background(), "/test.Service/Method", &emptypb.Empty{}, &emptypb.Empty{}, nil, nil)
	assert.ErrorContains(t, err, "extension failed")
}

func Test_sshTunnelOpt_BadResponse(t *testing.T) {
	ma := &mockExtendedAgent{
		extensionFunc: func(extensionType string, contents []byte) ([]byte, error) {
			return []byte("not-valid-ssh-wire-format"), nil
		},
	}
	interceptor := interceptorFromAgent(ma)
	err := interceptor(context.Background(), "/test.Service/Method", &emptypb.Empty{}, &emptypb.Empty{}, nil, nil)
	assert.Error(t, err)
}

func Test_sshTunnelOpt_StreamInterceptorNil(t *testing.T) {
	ma := &mockExtendedAgent{extensionFunc: func(string, []byte) ([]byte, error) { return nil, nil }}
	_, stream := sshAgentOpt(ma)(nil)
	assert.Nil(t, stream)
}
