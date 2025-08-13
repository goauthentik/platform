package pam

import (
	"context"

	"github.com/golang-jwt/jwt/v5"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (pam *Server) TokenAuth(ctx context.Context, req *pb.TokenAuthRequest) (*pb.TokenAuthResponse, error) {
	t, err := jwt.ParseWithClaims(req.Token, &token.AuthentikClaims{}, pam.kf.Keyfunc)
	if err != nil {
		return &pb.TokenAuthResponse{Successful: false}, err
	}

	token := token.Token{AccessToken: t}

	return &pb.TokenAuthResponse{
		Successful: true,
		Token: &pb.Token{
			PreferredUsername: token.Claims().Username,
			Iss:               token.Claims().Issuer,
			Sub:               token.Claims().Subject,
			Aud:               token.Claims().Audience,
			Exp:               timestamppb.New(token.Claims().ExpiresAt.Time),
			Nbf:               timestamppb.New(token.Claims().NotBefore.Time),
			Iat:               timestamppb.New(token.Claims().IssuedAt.Time),
			Jti:               token.Claims().ID,
		},
	}, nil
}
