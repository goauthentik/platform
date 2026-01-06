package auth

import (
	"context"

	"github.com/MicahParks/jwkset"
	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/mitchellh/mapstructure"
	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/agent_system/session"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (auth *Server) validateToken(ctx context.Context, rawToken string) (*token.Token, error) {
	var st jwkset.Storage
	jw := jwkset.JWKSMarshal{}
	_, dom, err := auth.ctx.DomainAPI()
	if err != nil {
		return nil, err
	}
	err = mapstructure.Decode(dom.Config().JwksAuth, &jw)
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
	if token.Claims().Audience[0] != dom.Config().DeviceId {
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

	sm := auth.ctx.GetComponent(session.ID).(*session.Server)
	if sm == nil {
		return nil, status.Error(codes.Internal, "cant find session component")
	}

	sess, err := sm.NewSession(ctx, session.SessionRequest{
		Username: req.Username,
		RawToken: req.Token,
		Token:    token,
	})
	if err != nil {
		return nil, status.Error(codes.NotFound, "unable to create session")
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
		SessionId: sess.ID,
	}, nil
}
