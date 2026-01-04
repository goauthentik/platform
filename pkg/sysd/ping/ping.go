package ping

import (
	"context"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/sysd/component"
	"goauthentik.io/platform/pkg/sysd/types"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

const ID = "ping"

type Server struct {
	pb.UnimplementedPingServer
	log *log.Entry
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
	}
	return srv, nil
}

func (ds *Server) Start() error {
	return nil
}

func (ds *Server) Stop() error {
	return nil
}

func (ds *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterPingServer(s, ds)
}

func (ds *Server) Ping(context.Context, *emptypb.Empty) (*pb.PingResponse, error) {
	return &pb.PingResponse{
		Component: "sysd",
		Version:   meta.FullVersion(),
	}, nil
}
