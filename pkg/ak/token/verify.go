package token

import (
	"context"

	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
)

type VerifyOpts struct {
	JWKSUrl string
}

func DefaultVerifyOpts(clientID string) VerifyOpts {
	return VerifyOpts{}
}

func VerifyToken(rawToken string, opts VerifyOpts) (*Token, error) {
	k, err := keyfunc.NewDefaultCtx(context.Background(), []string{opts.JWKSUrl})
	if err != nil {
		return nil, err
	}
	t, err := jwt.ParseWithClaims(
		rawToken,
		&AuthentikClaims{},
		k.Keyfunc,
	)
	if err != nil {
		return nil, err
	}
	ct := Token{
		AccessToken:    t,
		RawAccessToken: rawToken,
	}
	return &ct, nil
}
