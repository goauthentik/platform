package agent

import (
	"context"
	"errors"
	"fmt"
	"time"

	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
)

func (a *Agent) CachedTokenExchange(ctx context.Context, req *pb.TokenExchangeRequest) (*pb.TokenExchangeResponse, error) {
	prof, ok := a.cfg.Get().Profiles[req.Header.Profile]
	if !ok {
		return nil, errors.New("profile not found")
	}
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  pstr.S(fmt.Sprintf("authorize access to your account '%s' in '%s'", req.ClientId, creds.Parent.Cmdline)),
				Windows: pstr.S(fmt.Sprintf("'%s' is attempting to access your account in '%s'", req.ClientId, creds.Parent.Cmdline)),
				Linux:   pstr.S(fmt.Sprintf("'%s' is attempting to access your account in '%s'", req.ClientId, creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", req.ClientId, creds.UniqueProcessID()), nil
		},
		TimeoutSuccessful: time.Minute * 30,
		TimeoutDenied:     time.Second,
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
