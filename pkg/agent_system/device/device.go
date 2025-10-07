package device

import (
	"context"
	"errors"

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
	srv := &Server{
		log: ctx.Log,
		ctx: ctx.Context,
	}
	return srv, nil
}

func (ds *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	ac, err := config.Manager().Get().Domains()[0].APIClient()
	if err != nil {
		return err
	}
	ds.api = ac
	return nil
}

func (ds *Server) Stop() error {
	return nil
}

func (ds *Server) Register(grpc.ServiceRegistrar) {}
