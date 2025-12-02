package device

import (
	"context"
	"time"

	"github.com/MicahParks/jwkset"
	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/mitchellh/mapstructure"
	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func (ds *Server) validateChallenge(ctx context.Context, rawToken string) (*token.Token, error) {
	var st jwkset.Storage
	jw := jwkset.JWKSMarshal{}
	err := mapstructure.Decode(ds.dom.Config().JwksChallenge, &jw)
	if err != nil {
		return nil, err
	}
	sst, err := jw.ToStorage()
	if err != nil {
		return nil, err
	}
	st = sst

	k, err := keyfunc.New(keyfunc.Options{Storage: st, Ctx: ctx})
	if err != nil {
		return nil, errors.Wrap(err, "failed to create keyfunc")
	}
	t, err := jwt.ParseWithClaims(rawToken, &token.AuthentikClaims{}, k.Keyfunc)
	if err != nil {
		return nil, errors.Wrap(err, "failed to validate token")
	}

	token := token.Token{
		AccessToken:    t,
		RawAccessToken: rawToken,
	}
	return &token, nil
}

func (ds *Server) SignedEndpointHeader(ctx context.Context, req *pb.PlatformEndpointRequest) (*pb.PlatformEndpointResponse, error) {
	_, err := ds.validateChallenge(ctx, req.Challenge)
	if err != nil {
		return nil, err
	}
	hw, err := hardware.Gather()
	if err != nil {
		return nil, err
	}
	iat := time.Now()
	nt := jwt.NewWithClaims(jwt.SigningMethodHS512, jwt.MapClaims{
		"iss": hw.Serial,
		"aud": "goauthentik.io/platform/endpoint",
		"atc": req.Challenge,
		"iat": iat,
		"exp": iat.Add(5 * time.Minute),
	})
	s, err := nt.SignedString([]byte(config.Manager().Get().Domains()[0].Token))
	if err != nil {
		return nil, err
	}
	return &pb.PlatformEndpointResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Message: s,
	}, nil
}
