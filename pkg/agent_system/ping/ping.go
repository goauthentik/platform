package ping

import (
	"context"

	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

const ID = "ping"

type Server struct {
	pb.UnimplementedPingServer

	ctx component.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		ctx: ctx,
	}
	return srv, nil
}

func (ping *Server) Start() error {
	return nil
}

func (ping *Server) Stop() error {
	return nil
}

func (ping *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterPingServer(s, ping)
}

func (ping *Server) Ping(ctx context.Context, _ *emptypb.Empty) (*pb.PingResponse, error) {
	d, _, err := ping.ctx.DomainAPI()
	res := &pb.PingResponse{
		Component: "sysd",
		Version:   meta.FullVersion(),
	}
	if err != nil {
		ping.ctx.Log().WithError(err).Warning("failed to get domain API")
		return res, nil
	}
	v, _, err := d.AdminApi.AdminVersionRetrieve(ctx).Execute()
	if err != nil {
		ping.ctx.Log().WithError(err).Warning("failed to get authentik version")
		return res, nil
	}
	res.ServerVersion = v.VersionCurrent
	return res, nil
}
