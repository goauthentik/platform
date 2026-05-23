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

func Test_parseString_TooShort(t *testing.T) {
	_, _, ok := parseString([]byte{0, 0, 0})
	assert.False(t, ok)
}

func Test_parseString_EmptyString(t *testing.T) {
	in := []byte{0, 0, 0, 0, 'r', 'e', 's', 't'}
	out, rest, ok := parseString(in)
	assert.True(t, ok)
	assert.Empty(t, out)
	assert.Equal(t, []byte("rest"), rest)
}

func Test_parseString_Valid(t *testing.T) {
	in := append([]byte{0, 0, 0, 5}, []byte("hello")...)
	in = append(in, []byte("trailing")...)
	out, rest, ok := parseString(in)
	assert.True(t, ok)
	assert.Equal(t, []byte("hello"), out)
	assert.Equal(t, []byte("trailing"), rest)
}

func Test_parseString_LengthExceedsData(t *testing.T) {
	in := []byte{0, 0, 0, 10, 'a', 'b', 'c'}
	_, _, ok := parseString(in)
	assert.False(t, ok)
}

func Test_parseSignatureBody_Valid(t *testing.T) {
	sig := &ssh.Signature{Format: "ssh-ed25519", Blob: []byte("fake-blob")}
	data := ssh.Marshal(sig)
	out, rest, ok := parseSignatureBody(data)
	assert.True(t, ok)
	assert.Equal(t, "ssh-ed25519", out.Format)
	assert.Equal(t, []byte("fake-blob"), out.Blob)
	assert.Empty(t, rest)
	assert.Nil(t, out.Rest)
}

func Test_parseSignatureBody_SKAlgorithm(t *testing.T) {
	extra := []byte("app-specific-data")
	sig := &ssh.Signature{Format: ssh.KeyAlgoSKED25519, Blob: []byte("blob")}
	data := append(ssh.Marshal(sig), extra...)
	out, rest, ok := parseSignatureBody(data)
	assert.True(t, ok)
	assert.Equal(t, ssh.KeyAlgoSKED25519, out.Format)
	assert.Nil(t, rest)
	assert.Equal(t, extra, out.Rest)
}

func Test_parseSignatureBody_Malformed(t *testing.T) {
	out, _, ok := parseSignatureBody([]byte{})
	assert.False(t, ok)
	assert.Nil(t, out)
}
