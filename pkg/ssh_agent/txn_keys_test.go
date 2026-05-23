package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"testing"

	"github.com/golang-jwt/jwt/v5"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak/token"
	"golang.org/x/crypto/ssh"
)

var (
	pub  ssh.PublicKey
	priv ssh.Signer
)

func init() {
	_pub, _priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		panic(err)
	}
	priv, err = ssh.NewSignerFromKey(_priv)
	if err != nil {
		panic(err)
	}
	pub, err = ssh.NewPublicKey(_pub)
	if err != nil {
		panic(err)
	}
}

func Test_TXN_GenerateCert(t *testing.T) {
	txn := &AgentTxn{
		ag:      &Agent{},
		hostKey: pub,
	}
	crt, sig, err := txn.generateCert(token.Token{
		AccessToken: &jwt.Token{
			Claims: &token.AuthentikClaims{},
		},
	}, &api.AgentTokenResponse{
		ExpiresIn: api.PtrInt32(3600),
		Token:     "foo",
	})
	assert.NoError(t, err)
	assert.NotNil(t, sig)
	assert.NotNil(t, crt)
}

func Test_generateSSHPrivateKey(t *testing.T) {
	signer, err := generateSSHPrivateKey()
	assert.NoError(t, err)
	assert.NotNil(t, signer)
	assert.Equal(t, ssh.KeyAlgoED25519, signer.PublicKey().Type())
}

func Test_TXN_GenerateCert_Fields(t *testing.T) {
	const testUsername = "alice"
	txn := &AgentTxn{ag: &Agent{}, hostKey: pub}
	crt, _, err := txn.generateCert(token.Token{
		AccessToken: &jwt.Token{
			Claims: &token.AuthentikClaims{Username: testUsername},
		},
	}, &api.AgentTokenResponse{
		ExpiresIn: api.PtrInt32(3600),
		Token:     "host-token-value",
	})
	assert.NoError(t, err)
	assert.Equal(t, uint32(ssh.UserCert), crt.CertType)
	assert.Equal(t, []string{testUsername}, crt.ValidPrincipals)
	assert.Equal(t, testUsername, crt.KeyId)
	assert.Greater(t, crt.ValidBefore, uint64(0))
	assert.Equal(t, "host-token-value", crt.Extensions[ExtAuthentikPlatformSSHToken])
	assert.Contains(t, crt.Extensions, "permit-pty")
	assert.Contains(t, crt.Extensions, "permit-agent-forwarding")
	assert.Contains(t, crt.Extensions, ExtAuthentikPlatformSSHHostKey)
}
