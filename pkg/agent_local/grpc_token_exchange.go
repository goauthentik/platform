package agentlocal

import (
	"context"

	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
)

func (a *Agent) CachedTokenExchange(ctx context.Context, req *pb.TokenExchangeRequest) (*pb.TokenExchangeResponse, error) {
	prof := a.cfg.Get().Profiles[req.Header.Profile]
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
