package agentlocal

import (
	"context"
	"errors"
	"fmt"
	"time"

	authzprompt "goauthentik.io/cli/pkg/agent_local/authz_prompt"
	"goauthentik.io/cli/pkg/agent_local/grpc_creds"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
)

func (a *Agent) CachedTokenExchange(ctx context.Context, req *pb.TokenExchangeRequest) (*pb.TokenExchangeResponse, error) {
	prof, ok := a.cfg.Get().Profiles[req.Header.Profile]
	if !ok {
		return nil, errors.New("profile not found")
	}
	if err := a.authorizeRequest(ctx, req.Header.Profile, authzprompt.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("Application '%s' is attempting to get a token for '%s'", creds.ParentCmdline, req.ClientId), nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", req.ClientId, creds.UniqueProcessID()), nil
		},
		Timeout: func() time.Duration {
			return time.Minute * 30
		},
	}); err != nil {
		return nil, err
	}
	nt, err := token.CachedExchangeToken(req.Header.Profile, prof, token.DefaultExchangeOpts(req.ClientId))
	if err != nil {
		a.log.WithError(err).Warn("failed to exchange token")
		return nil, err
	}
	a.log.WithField("clientId", req.ClientId).Debug("Exchanged token")
	return &pb.TokenExchangeResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		AccessToken: nt.RawAccessToken,
	}, nil
}
