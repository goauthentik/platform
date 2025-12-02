package device

import (
	"context"
	"errors"
	"time"

	"github.com/MicahParks/jwkset"
	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/mitchellh/mapstructure"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func (ds *Server) validateChallenge(ctx context.Context, rawToken string) (*token.Token, *config.DomainConfig, error) {
	for _, dom := range config.Manager().Get().Domains() {
		var st jwkset.Storage
		jw := jwkset.JWKSMarshal{}
		err := mapstructure.Decode(dom.Config().JwksChallenge, &jw)
		if err != nil {
			ds.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to load config")
			continue
		}
		sst, err := jw.ToStorage()
		if err != nil {
			ds.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to parse jwks")
			continue
		}
		st = sst

		k, err := keyfunc.New(keyfunc.Options{Storage: st, Ctx: ctx})
		if err != nil {
			ds.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to create keyfunc")
			continue
		}
		t, err := jwt.ParseWithClaims(rawToken, &token.AuthentikClaims{}, k.Keyfunc)
		if err != nil {
			ds.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to validate token")
			continue
		}
		token := token.Token{
			AccessToken:    t,
			RawAccessToken: rawToken,
		}
		return &token, dom, nil
	}
	return nil, nil, errors.New("could not find matching domain")
}

func (ds *Server) SignedEndpointHeader(ctx context.Context, req *pb.PlatformEndpointRequest) (*pb.PlatformEndpointResponse, error) {
	_, dom, err := ds.validateChallenge(ctx, req.Challenge)
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
	s, err := nt.SignedString([]byte(dom.Token))
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
