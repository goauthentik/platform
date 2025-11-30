package agentlocal

import (
	"context"
	"fmt"
	"time"

	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (a *Agent) GetCurrentToken(ctx context.Context, req *pb.CurrentTokenRequest) (*pb.CurrentTokenResponse, error) {
	pfm := a.tr.ForProfile(req.Header.Profile)
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  pstr.S(fmt.Sprintf("authorize access to your account in '%s'", creds.Parent.Cmdline)),
				Windows: pstr.S(fmt.Sprintf("'%s' is attempting to access your account.", creds.Parent.Cmdline)),
				Linux:   pstr.S(fmt.Sprintf("'%s' is attempting to access your account.", creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", creds.UniqueProcessID(), req.Type), nil
		},
		Timeout: func() time.Duration {
			return time.Hour * 2
		},
	}); err != nil {
		return nil, err
	}
	var token token.Token
	var err error
	switch req.Type {
	case pb.CurrentTokenRequest_UNVERIFIED:
		token, err = pfm.Unverified()
	case pb.CurrentTokenRequest_VERIFIED:
		token, err = pfm.Token()
	case pb.CurrentTokenRequest_UNSPECIFIED:
		return nil, fmt.Errorf("unsupported token type: %s", req.Type)
	}
	if err != nil {
		return nil, err
	}
	return &pb.CurrentTokenResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Token: &pb.Token{
			PreferredUsername: token.Claims().Username,
			Iss:               token.Claims().Issuer,
			Sub:               token.Claims().Subject,
			Aud:               token.Claims().Audience,
			Exp:               timestamppb.New(token.Claims().ExpiresAt.Time),
			Iat:               timestamppb.New(token.Claims().IssuedAt.Time),
			Jti:               token.Claims().ID,
		},
		Raw: token.RawAccessToken,
		Url: a.cfg.Get().Profiles[req.Header.Profile].AuthentikURL,
	}, nil
}

func (a *Agent) Authorize(ctx context.Context, req *pb.AuthorizeRequest) (*pb.AuthorizeResponse, error) {
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  pstr.S(fmt.Sprintf("authorize access to '%s'", req.Service)),
				Windows: pstr.S(fmt.Sprintf("'%s' is requesting access.", creds.Parent.Cmdline)),
				Linux:   pstr.S(fmt.Sprintf("'%s' is requesting access.", creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return req.Uid, nil
		},
		Timeout: func() time.Duration {
			return 3600 * time.Second
		},
	}); err != nil {
		return nil, err
	}
	return &pb.AuthorizeResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
	}, nil
}
