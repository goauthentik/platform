package sshagent

// Based on https://github.com/drakkan/agent/

import (
	"encoding/binary"
	"errors"
	"fmt"

	"golang.org/x/crypto/ssh"
)

const (
	ExtOpenSSHSessionBind = "session-bind@openssh.com"
)

// ParseSessionBind parses the payload of a "session-bind@openssh.com" extension
// message. It verifies the signature within the bind message to ensure the
// integrity of the hop.
func ParseSessionBind(data []byte) (SessionBind, error) {
	var tmp struct {
		HostKeyBlob       []byte
		SessionIdentifier []byte
		Signature         []byte
		IsForwarding      bool
	}

	if err := ssh.Unmarshal(data, &tmp); err != nil {
		return SessionBind{}, err
	}
	hostKey, err := ssh.ParsePublicKey(tmp.HostKeyBlob)
	if err != nil {
		return SessionBind{}, err
	}

	sig, rest, ok := parseSignatureBody(tmp.Signature)
	if len(rest) > 0 || !ok {
		return SessionBind{}, errors.New("ssh: signature parse error")
	}
	// See process_ext_session_bind
	if len(tmp.SessionIdentifier) > 128 {
		return SessionBind{}, fmt.Errorf("ssh: session bind sid len %d, max allowed 128", len(tmp.SessionIdentifier))
	}
	if err := hostKey.Verify(tmp.SessionIdentifier, sig); err != nil {
		return SessionBind{}, err
	}

	return SessionBind{
		HostKey:    hostKey,
		SessionID:  tmp.SessionIdentifier,
		Forwarding: tmp.IsForwarding,
	}, nil
}

// SessionBind represents a single hop in the agent forwarding chain, as defined
// by the "session-bind@openssh.com" extension in [PROTOCOL.agent].
type SessionBind struct {
	HostKey    ssh.PublicKey
	SessionID  []byte
	Forwarding bool
}

func parseSignatureBody(in []byte) (out *ssh.Signature, rest []byte, ok bool) {
	format, in, ok := parseString(in)
	if !ok {
		return
	}

	out = &ssh.Signature{
		Format: string(format),
	}

	if out.Blob, in, ok = parseString(in); !ok {
		return
	}

	switch out.Format {
	case ssh.KeyAlgoSKECDSA256, ssh.CertAlgoSKECDSA256v01, ssh.KeyAlgoSKED25519, ssh.CertAlgoSKED25519v01:
		out.Rest = in
		return out, nil, ok
	}

	return out, in, ok
}

func parseString(in []byte) (out, rest []byte, ok bool) {
	if len(in) < 4 {
		return
	}
	length := binary.BigEndian.Uint32(in)
	in = in[4:]
	if uint32(len(in)) < length {
		return
	}
	out = in[:length]
	rest = in[length:]
	ok = true
	return
}
