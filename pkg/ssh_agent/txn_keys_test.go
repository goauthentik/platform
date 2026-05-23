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
