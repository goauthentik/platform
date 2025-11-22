package auth

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"time"

	"github.com/MicahParks/jwkset"
	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	"github.com/gorilla/securecookie"
	"github.com/pkg/errors"
	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (auth *Server) startFetch() {
	d := time.Second * time.Duration(auth.dom.Config().RefreshInterval)
	auth.log.Info("Starting initial JWKS fetch")
	auth.refreshTokenJWKS()
	auth.log.WithField("next", d.String()).Info("Finished initial JWKS fetch")
	t := time.NewTimer(d)
	go func() {
		for {
			select {
			case <-t.C:
				auth.log.Info("Starting JWKS fetch")
				auth.refreshTokenJWKS()
				auth.log.WithField("next", d.String()).Info("Finished JWKS fetch")
			case <-auth.ctx.Context().Done():
				return
			}
		}
	}()
}

func (auth *Server) refreshTokenJWKS() {
	jwk, err := jwkset.NewStorageFromHTTP(auth.dom.Config().JwksUrl, jwkset.HTTPClientStorageOptions{})
	if err != nil {
		auth.log.WithError(err).Warning("failed to fetch JWKS")
		return
	}
	err = auth.ctx.StateForDomain(auth.dom).Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		jwk, err := jwk.Marshal(context.Background())
		if err != nil {
			return err
		}
		r, err := json.Marshal(jwk)
		if err != nil {
			return err
		}
		return b.Put([]byte("jwks"), r)
	})
	if err != nil {
		auth.log.WithError(err).Warning("failed to save updated JWKS")
	}
}

func (auth *Server) validateToken(rawToken string) (*token.Token, error) {
	var st jwkset.Storage
	err := auth.ctx.StateForDomain(auth.dom).View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		r := b.Get([]byte("jwks"))
		jw := jwkset.JWKSMarshal{}
		err := json.Unmarshal(r, &jw)
		if err != nil {
			return err
		}
		sst, err := jw.ToStorage()
		if err != nil {
			return err
		}
		st = sst
		return nil
	})
	if err != nil {
		return nil, errors.Wrap(err, "failed to check JWKS")
	}

	k, _ := keyfunc.New(keyfunc.Options{Storage: st})
	t, err := jwt.ParseWithClaims(rawToken, &token.AuthentikClaims{}, k.Keyfunc)
	if err != nil {
		return nil, errors.Wrap(err, "failed to validate token")
	}

	token := token.Token{AccessToken: t}
	return &token, nil
}

func (auth *Server) TokenAuth(ctx context.Context, req *pb.TokenAuthRequest) (*pb.TokenAuthResponse, error) {
	token, err := auth.validateToken(req.Token)
	if err != nil {
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
