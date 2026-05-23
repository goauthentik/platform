package sshagent

import (
	"net"
	"testing"

	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak/token"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
)

func newTestTxn() *AgentTxn {
	return &AgentTxn{
		ag:  &Agent{},
		log: log.WithField("logger", "test"),
	}
}

// newTestToken returns a minimal token fixture matching the pattern in txn_keys_test.go.
func newTestToken() token.Token {
	return token.Token{
		AccessToken: &jwt.Token{
			Claims: &token.AuthentikClaims{},
		},
	}
}

// newTestHostToken returns a minimal host token fixture.
func newTestHostToken() *api.AgentTokenResponse {
	return &api.AgentTokenResponse{
		ExpiresIn: api.PtrInt32(3600),
		Token:     "test-token",
	}
}

func Test_AgentTxn_StubMethods(t *testing.T) {
	txn := newTestTxn()
	assert.NoError(t, txn.Add(agent.AddedKey{}))
	assert.NoError(t, txn.Remove(pub))
	assert.NoError(t, txn.RemoveAll())
	assert.NoError(t, txn.Lock([]byte("pass")))
	assert.NoError(t, txn.Unlock([]byte("pass")))
	sig, err := txn.Sign(pub, []byte("data"))
	assert.Nil(t, sig)
	assert.NoError(t, err)
	signers, err := txn.Signers()
	assert.NoError(t, err)
	assert.Empty(t, signers)
}

func Test_AgentTxn_List_WithCert(t *testing.T) {
	txn := &AgentTxn{
		ag:      &Agent{},
		log:     log.WithField("logger", "test"),
		hostKey: pub,
	}
	crt, signer, err := txn.generateCert(newTestToken(), newTestHostToken())
	assert.NoError(t, err)
	// Pre-set crt so ensureCert returns early without touching gtm.
	txn.crt = crt
	txn.cpk = signer

	keys, err := txn.List()
	assert.NoError(t, err)
	assert.Len(t, keys, 1)
	assert.Equal(t, crt.Type(), keys[0].Format)
}

func Test_AgentTxn_SignWithFlags_NilKey(t *testing.T) {
	txn := newTestTxn()
	// Pre-set a dummy crt so ensureCert returns early; cpk stays nil.
	txn.crt = &ssh.Certificate{Key: pub}
	sig, err := txn.SignWithFlags(pub, []byte("data"), 0)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "no key for host")
	assert.Nil(t, sig)
}

func Test_AgentTxn_SignWithFlags_WithKey(t *testing.T) {
	txn := newTestTxn()
	// Pre-set crt and cpk so ensureCert is short-circuited.
	txn.crt = &ssh.Certificate{Key: pub}
	txn.cpk = priv
	sig, err := txn.SignWithFlags(pub, []byte("data to sign"), 0)
	assert.NoError(t, err)
	assert.NotNil(t, sig)
}

func Test_AgentTxn_Extension_Unknown(t *testing.T) {
	txn := newTestTxn()
	result, err := txn.Extension("unknown-extension@example.com", []byte("payload"))
	assert.NoError(t, err)
	assert.Equal(t, []byte{}, result)
}

func Test_AgentTxn_Extension_SessionBind_Valid(t *testing.T) {
	txn := newTestTxn()
	sessionID := []byte("test-session-identifier")
	payload := makeSessionBindPayload(t, sessionID, priv, false)

	result, err := txn.Extension(ExtOpenSSHSessionBind, payload)
	assert.NoError(t, err)
	assert.Equal(t, []byte{}, result)
	assert.Equal(t, sessionID, txn.sshSessionID)
	assert.NotNil(t, txn.hostKey)
}

func Test_AgentTxn_Extension_SessionBind_Invalid(t *testing.T) {
	txn := newTestTxn()
	_, err := txn.Extension(ExtOpenSSHSessionBind, []byte("not valid ssh data"))
	assert.Error(t, err)
}

func Test_AgentTxn_Close_NilTunnel(t *testing.T) {
	txn := newTestTxn()
	assert.NoError(t, txn.Close())
}

func Test_AgentTxn_Close_WithTunnel(t *testing.T) {
	txn := newTestTxn()
	c1, c2 := net.Pipe()
	defer func() {
		assert.NoError(t, c2.Close())
	}()
	txn.tunnelConn = c1
	assert.NoError(t, txn.Close())
}
