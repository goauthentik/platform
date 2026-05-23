package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"testing"

	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/ssh"
)

// makeSessionBindPayload builds a well-formed session-bind@openssh.com payload
// signed by the given signer.
func makeSessionBindPayload(t *testing.T, sessionID []byte, signer ssh.Signer, forwarding bool) []byte {
	t.Helper()
	sig, err := signer.Sign(rand.Reader, sessionID)
	assert.NoError(t, err)
	return ssh.Marshal(struct {
		HostKeyBlob       []byte
		SessionIdentifier []byte
		Signature         []byte
		IsForwarding      bool
	}{
		HostKeyBlob:       signer.PublicKey().Marshal(),
		SessionIdentifier: sessionID,
		Signature:         ssh.Marshal(sig),
		IsForwarding:      forwarding,
	})
}

func Test_ParseSessionBind_Valid(t *testing.T) {
	sessionID := make([]byte, 32)
	_, err := rand.Read(sessionID)
	assert.NoError(t, err)

	payload := makeSessionBindPayload(t, sessionID, priv, false)
	sb, err := ParseSessionBind(payload)
	assert.NoError(t, err)
	assert.Equal(t, sessionID, sb.SessionID)
	assert.False(t, sb.Forwarding)
	assert.NotNil(t, sb.HostKey)
}

func Test_ParseSessionBind_ForwardingFlag(t *testing.T) {
	sessionID := []byte("test-session")
	payload := makeSessionBindPayload(t, sessionID, priv, true)
	sb, err := ParseSessionBind(payload)
	assert.NoError(t, err)
	assert.True(t, sb.Forwarding)
}

func Test_ParseSessionBind_InvalidSignature(t *testing.T) {
	// Sign with a different key than the one in HostKeyBlob.
	_, otherPriv, err := ed25519.GenerateKey(rand.Reader)
	assert.NoError(t, err)
	otherSigner, err := ssh.NewSignerFromKey(otherPriv)
	assert.NoError(t, err)

	sessionID := []byte("test-session")
	sig, err := otherSigner.Sign(rand.Reader, sessionID)
	assert.NoError(t, err)

	payload := ssh.Marshal(struct {
		HostKeyBlob       []byte
		SessionIdentifier []byte
		Signature         []byte
		IsForwarding      bool
	}{
		HostKeyBlob:       pub.Marshal(), // from priv, but sig is from otherSigner
		SessionIdentifier: sessionID,
		Signature:         ssh.Marshal(sig),
		IsForwarding:      false,
	})
	_, err = ParseSessionBind(payload)
	assert.Error(t, err)
}

func Test_ParseSessionBind_TooLongSessionID(t *testing.T) {
	sessionID := make([]byte, 129)
	payload := makeSessionBindPayload(t, sessionID, priv, false)
	_, err := ParseSessionBind(payload)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "128")
}

func Test_ParseSessionBind_MalformedData(t *testing.T) {
	_, err := ParseSessionBind([]byte("not valid ssh marshal data"))
	assert.Error(t, err)
}
