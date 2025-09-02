package agentlocal

import (
	"context"
	"fmt"
	"time"

	authzprompt "goauthentik.io/cli/pkg/agent_local/authz_prompt"
	"goauthentik.io/cli/pkg/agent_local/grpc_creds"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (a *Agent) GetCurrentToken(ctx context.Context, req *pb.CurrentTokenRequest) (*pb.CurrentTokenResponse, error) {
	pfm := a.tr.ForProfile(req.Header.Profile)
	if err := a.authorizeRequest(ctx, req.Header.Profile, authzprompt.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("Application '%s' is attempting to access you token", creds.ParentCmdline), nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", creds.UniqueProcessID(), req.Type), nil
		},
		Timeout: func() time.Duration {
			return time.Minute * 30
		},
	}); err != nil {
		return nil, err
	}
	var token token.Token
	switch req.Type {
	case pb.CurrentTokenRequest_UNVERIFIED:
		token = pfm.Unverified()
	case pb.CurrentTokenRequest_VERIFIED:
		token = pfm.Token()
	case pb.CurrentTokenRequest_UNSPECIFIED:
		return nil, fmt.Errorf("unsupported token type: %s", req.Type)
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
		Url: storage.Manager().Get().Profiles[req.Header.Profile].AuthentikURL,
	}, nil
}
