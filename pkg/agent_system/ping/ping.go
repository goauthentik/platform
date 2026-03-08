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
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{}
	return srv, nil
}

func (ps *Server) Start() error {
	return nil
}

func (ps *Server) Stop() error {
	return nil
}

func (ps *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterPingServer(s, ps)
}

func (ps *Server) Ping(context.Context, *emptypb.Empty) (*pb.PingResponse, error) {
	return &pb.PingResponse{
		Component: "sysd",
		Version:   meta.FullVersion(),
	}, nil
}
