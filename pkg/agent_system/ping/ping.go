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

func (ping *Server) Ping(context.Context, *emptypb.Empty) (*pb.PingResponse, error) {
	return &pb.PingResponse{
		Component: "sysd",
		Version:   meta.FullVersion(),
	}, nil
}
