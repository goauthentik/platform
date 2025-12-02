package auth

import (
	"context"
	"encoding/base64"

	"github.com/MicahParks/jwkset"
	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/gorilla/securecookie"
	"github.com/mitchellh/mapstructure"
	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (auth *Server) validateToken(ctx context.Context, rawToken string) (*token.Token, error) {
	var st jwkset.Storage
	jw := jwkset.JWKSMarshal{}
	err := mapstructure.Decode(auth.dom.Config().JwksAuth, &jw)
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
	if token.Claims().Audience[0] != auth.dom.Config().DeviceId {
		return nil, errors.New("token not for device")
	}
	return &token, nil
}

func (auth *Server) TokenAuth(ctx context.Context, req *pb.TokenAuthRequest) (*pb.TokenAuthResponse, error) {
	token, err := auth.validateToken(ctx, req.Token)
	if err != nil {
		auth.log.WithError(err).Warning("failed to validate token")
		return nil, err
	}

	return &pb.TokenAuthResponse{
		Successful: true,
		Token: &pb.Token{
			PreferredUsername: token.Claims().Username,
			Iss:               token.Claims().Issuer,
			Sub:               token.Claims().Subject,
			Aud:               token.Claims().Audience,
			Exp:               timestamppb.New(token.Claims().ExpiresAt.Time),
			Iat:               timestamppb.New(token.Claims().IssuedAt.Time),
			Jti:               token.Claims().ID,
		},
		SessionId: base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64)),
	}, nil
}
