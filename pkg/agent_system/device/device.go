package device

import (
	"context"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedNSSServer

	api *api.APIClient
	log *log.Entry

	ctx context.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	ac, err := config.Manager().Get().Domains()[0].APIClient()
	if err != nil {
		return nil, err
	}
	srv := &Server{
		api: ac,
		log: ctx.Log,
		ctx: ctx.Context,
	}
	return srv, nil
}

func (ds *Server) Start() {}

func (ds *Server) Stop() error {
	return nil
}

func (ds *Server) Register(grpc.ServiceRegistrar) {}
