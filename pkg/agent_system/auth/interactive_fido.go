package auth

import (
	"encoding/base64"
	"encoding/json"

	"github.com/mitchellh/mapstructure"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/proto"
)

type webauthnChallenge struct {
	Challenge          string `mapstructure:"challenge"`
	Timeout            int    `mapstructure:"timeout"`
	RelyingPartyID     string `mapstructure:"rpId"`
	AllowedCredentials []struct {
		CredentialID string `mapstructure:"id"`
	} `mapstructure:"allowCredentials"`
	UserVerification string `mapstructure:"userVerification"`
}

const (
	uvPreferred   = "preferred"
	uvRequired    = "required"
	uvDiscouraged = "discouraged"
)

func encodeChallenge(challenge string, origin string) ([]byte, error) {
	clientDataJSON := map[string]any{
		"type":      "webauthn.get",
		"challenge": challenge,
		"origin":    origin,
	}

	clientDataJSONBytes, err := json.Marshal(clientDataJSON)
	if err != nil {
		return nil, err
	}
	return clientDataJSONBytes, nil
}

func (txn *InteractiveAuthTransaction) parseWebAuthNRequest(dc api.DeviceChallenge) (*pb.InteractiveChallenge, error) {
	vv := webauthnChallenge{}
	err := mapstructure.Decode(dc.Challenge, &vv)
	if err != nil {
		return nil, err
	}

	challenge, err := encodeChallenge(vv.Challenge, txn.dom.AuthentikURL)
	if err != nil {
		txn.log.WithError(err).Warning("failed to encode challenge")
		return nil, err
	}

	bc := &pb.FIDORequest{
		RpId:          vv.RelyingPartyID,
		Challenge:     challenge,
		CredentialIds: [][]byte{},
	}
	if vv.UserVerification == uvPreferred || vv.UserVerification == uvRequired {
		bc.Uv = true
	}
	for _, dev := range vv.AllowedCredentials {
		credId, err := base64.RawURLEncoding.DecodeString(dev.CredentialID)
		if err != nil {
			txn.log.WithError(err).Warning("failed to decode device ID")
			return nil, err
		}
		bc.CredentialIds = append(bc.CredentialIds, credId)
	}
	qer, err := proto.Marshal(bc)
	if err != nil {
		txn.log.WithError(err).Warning("failed to marshall proto message")
		return nil, err
	}
	return &pb.InteractiveChallenge{
		Txid:       txn.ID,
		Prompt:     base64.StdEncoding.EncodeToString(qer),
		PromptMeta: pb.InteractiveChallenge_PAM_BINARY_PROMPT,
		Component:  string(flow.StageAuthenticatorValidate),
	}, nil
}

func (txn *InteractiveAuthTransaction) parseWebAuthNResponse(raw string, dc api.DeviceChallenge) (*api.AuthenticatorValidationChallengeResponseRequest, error) {
	d, err := base64.StdEncoding.DecodeString(raw)
	if err != nil {
		return nil, err
	}
	var m pb.FIDOResponse
	err = proto.Unmarshal(d, &m)
	if err != nil {
		return nil, err
	}

	vv := webauthnChallenge{}
	err = mapstructure.Decode(dc.Challenge, &vv)
	if err != nil {
		return nil, err
	}

	challenge, err := encodeChallenge(vv.Challenge, txn.dom.AuthentikURL)
	if err != nil {
		txn.log.WithError(err).Warning("failed to encode challenge")
		return nil, err
	}

	res := &api.AuthenticatorValidationChallengeResponseRequest{
		Component: api.PtrString(string(flow.StageAuthenticatorValidate)),
		Webauthn: map[string]any{
			"id":    base64.RawURLEncoding.EncodeToString(m.CredentialId),
			"rawId": base64.RawURLEncoding.EncodeToString(m.CredentialId),
			"type":  "public-key",
			"response": map[string]any{
				"clientDataJSON":    base64.RawURLEncoding.EncodeToString(challenge),
				"signature":         base64.RawURLEncoding.EncodeToString(m.Signature),
				"authenticatorData": base64.RawURLEncoding.EncodeToString(m.AuthenticatorData),
				"userHandle":        nil,
			},
		},
	}
	return res, nil
}
