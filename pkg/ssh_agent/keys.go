package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/pem"

	"golang.org/x/crypto/ssh"
)

var CA ssh.Signer

func init() {
	_ca, err := generateSSHPrivateKey()
	if err != nil {
		panic(err)
	}
	CA = _ca
}

func (ag *Agent) generateKey() (*ssh.Certificate, ssh.Signer, error) {
	tk, err := ag.gtm.ForProfile("default").Token()
	if err != nil {
		return nil, nil, err
	}

	key, err := generateSSHPrivateKey()
	if err != nil {
		return nil, nil, err
	}

	testCert := &ssh.Certificate{
		CertType:        ssh.UserCert,
		Nonce:           []byte{},
		ValidPrincipals: []string{tk.Claims().Username},
		ValidAfter:      0,
		ValidBefore:     ssh.CertTimeInfinity,
		Reserved:        []byte{},
		Key:             key.PublicKey(),
		KeyId:           "testcert",
		Permissions: ssh.Permissions{
			CriticalOptions: map[string]string{},
			Extensions:      map[string]string{},
			ExtraData: map[any]any{
				"ak-token": tk.RawAccessToken,
			},
		},
	}

	if err = testCert.SignCert(rand.Reader, key); err != nil {
		return nil, nil, err
	}
	return testCert, key, nil
}

func generateSSHPrivateKey() (ssh.Signer, error) {
	_, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	bytes, err := x509.MarshalPKCS8PrivateKey(priv)
	if err != nil {
		return nil, err
	}

	privatePem := pem.EncodeToMemory(
		&pem.Block{
			Type:  "PRIVATE KEY",
			Bytes: bytes,
		},
	)

	sshPriv, err := ssh.ParsePrivateKey(privatePem)
	if err != nil {
		return nil, err
	}
	return sshPriv, nil
}
