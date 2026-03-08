package device

import (
	"context"
	"errors"
	"time"

	"github.com/MicahParks/jwkset"
	"github.com/golang-jwt/jwt/v5"
	"github.com/mitchellh/mapstructure"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func parseChallengeToken(rawToken string, rawJWKS map[string]any) (*jwt.Token, error) {
	jw := jwkset.JWKSMarshal{}
	err := mapstructure.Decode(rawJWKS, &jw)
	if err != nil {
		return nil, err
	}
	keys, err := jw.JWKSlice()
	if err != nil {
		return nil, err
	}
	headerOnly := &token.AuthentikClaims{}
	parser := jwt.NewParser()
	unverified, _, err := parser.ParseUnverified(rawToken, headerOnly)
	if err != nil {
		return nil, err
	}
	targetKID, _ := unverified.Header["kid"].(string)
	var lastErr error
	for _, key := range keys {
		if targetKID != "" {
			marshaled := key.Marshal()
			if marshaled.KID != "" && marshaled.KID != targetKID {
				continue
			}
		}
		parsed, err := jwt.ParseWithClaims(rawToken, &token.AuthentikClaims{}, func(*jwt.Token) (any, error) {
			return key.Key(), nil
		})
		if err == nil {
			return parsed, nil
		}
		lastErr = err
	}
	if lastErr != nil {
		return nil, lastErr
	}
	return nil, errors.New("no matching jwk found for challenge")
}

func (ds *Server) validateChallenge(_ context.Context, rawToken string) (*token.Token, *config.DomainConfig, error) {
	for _, dom := range config.Manager().Get().Domains() {
		t, err := parseChallengeToken(rawToken, dom.Config().JwksChallenge)
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
	hw, err := hardware.Gather(common.New(ds.log, ctx))
	if err != nil {
		return nil, err
	}
	iat := time.Now()
	nt := jwt.NewWithClaims(jwt.SigningMethodHS512, jwt.MapClaims{
		"iss": hw.Serial,
		"aud": "goauthentik.io/platform/endpoint",
		"atc": req.Challenge,
		"iat": iat.Unix(),
		"exp": iat.Add(5 * time.Minute).Unix(),
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
