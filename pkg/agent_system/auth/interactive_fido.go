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
		return nil, err
	}
	vv := webauthnChallenge{}
	json.Unmarshal(v, &vv)

	challenge, err := base64.URLEncoding.DecodeString(vv.Challenge)
	if err != nil {
		return nil, err
	}

	bc := &pb.FIDORequest{
		RpId:          vv.RelyingPartyID,
		Challenge:     challenge,
		CredentialIds: [][]byte{},
	}
	for _, dev := range vv.AllowedCredentials {
		credId, err := base64.URLEncoding.DecodeString(dev.CredentialID)
		if err != nil {
			return nil, err
		}
		bc.CredentialIds = append(bc.CredentialIds, credId)
	}
	qer, err := proto.Marshal(bc)
	if err != nil {
		return nil, err
	}
	return &pb.InteractiveChallenge{
		Txid:       txn.ID,
		Prompt:     base64.StdEncoding.EncodeToString(qer),
		PromptMeta: pb.InteractiveChallenge_PAM_BINARY_PROMPT,
	}, nil
}

type webauthnResponseResponse struct {
	ClientDataJSON    string  `mapstructure:"clientDataJSON"`
	Signature         string  `mapstructure:"signature"`
	AuthenticatorData string  `mapstructure:"authenticatorData"`
	UserHandle        *string `mapstructure:"userHandle"`
}

type webauthnResponse struct {
	ID       string                   `mapstructure:"id"`
	RawID    string                   `mapstructure:"rawId"`
	Response webauthnResponseResponse `mapstructure:"response"`
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
	webauthn := map[string]any{}
	mapstructure.Decode(webauthnResponse{
		Response: webauthnResponseResponse{
			Signature:         base64.URLEncoding.EncodeToString(m.Signature),
			AuthenticatorData: base64.URLEncoding.EncodeToString(m.AuthenticatorData),
		},
	}, webauthn)
	res := &api.AuthenticatorValidationChallengeResponseRequest{
		Component: api.PtrString(string(flow.StageAuthenticatorValidate)),
		Webauthn:  webauthn,
	}
	return res, nil
}
