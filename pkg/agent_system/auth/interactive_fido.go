package auth

import (
	"encoding/base64"
	"encoding/json"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/proto"
)

type webauthnChallenge struct {
	Challenge          string `json:"challenge"`
	Timeout            int    `json:"timeout,omitempty"`
	RelyingPartyID     string `json:"rpId,omitempty"`
	AllowedCredentials []struct {
		CredentialID string `json:"id"`
	} `json:"allowCredentials,omitempty"`
}

func (txn *InteractiveAuthTransaction) parseWebAuthNRequest(dc api.DeviceChallenge) (*pb.InteractiveChallenge, error) {
	v, err := json.Marshal(dc.Challenge)
	if err != nil {
		txn.log.WithError(err).Warning("failed to marshall challenge")
		return nil, err
	}
	vv := webauthnChallenge{}
	json.Unmarshal(v, &vv)

	txn.log.Debugf("ch %+v\n", vv)

	challenge, err := base64.RawURLEncoding.DecodeString(vv.Challenge)
	if err != nil {
		txn.log.WithError(err).Warning("failed to decode challenge")
		return nil, err
	}

	bc := &pb.FIDORequest{
		RpId:          vv.RelyingPartyID,
		Challenge:     challenge,
		CredentialIds: [][]byte{},
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

func (txn *InteractiveAuthTransaction) parseWebAuthNResponse(raw string) (*api.AuthenticatorValidationChallengeResponseRequest, error) {
	d, err := base64.StdEncoding.DecodeString(raw)
	if err != nil {
		return nil, err
	}
	var m pb.FIDOResponse
	err = proto.Unmarshal(d, &m)
	if err != nil {
		return nil, err
	}

	res := &api.AuthenticatorValidationChallengeResponseRequest{
		Component: api.PtrString(string(flow.StageAuthenticatorValidate)),
		Webauthn: map[string]any{
			"id":    base64.RawURLEncoding.EncodeToString(m.CredentialId),
			"rawId": base64.RawURLEncoding.EncodeToString(m.CredentialId),
			"type":  "public-key",
			"response": map[string]any{
				"clientDataJSON":    "{}",
				"signature":         base64.RawURLEncoding.EncodeToString(m.Signature),
				"authenticatorData": base64.RawURLEncoding.EncodeToString(m.AuthenticatorData),
				"userHandle":        nil,
			},
		},
	}
	txn.log.Debugf("res %+v\n", res.Webauthn)
	return res, nil
}
