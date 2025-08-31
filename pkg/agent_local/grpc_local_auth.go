package agentlocal

import (
	"context"

	"github.com/pkg/errors"
	"goauthentik.io/cli/pkg/agent_local/grpc_creds"
	"google.golang.org/grpc"
	"google.golang.org/grpc/peer"
)

var (
	errFailedAuth = errors.New("failed to authorize")
)

func (a *Agent) AuthorizationUnaryInterceptor(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (resp any, err error) {
	p, ok := peer.FromContext(ctx)
	if !ok {
		return nil, errFailedAuth
	}
	creds := p.AuthInfo.(grpc_creds.AuthInfo).Creds
	cmd, err := creds.Parent.Cmdline()
	if err != nil {
		return nil, err
	}
	a.log.Debug(creds.PID, cmd)
	return handler(ctx, req)
}
