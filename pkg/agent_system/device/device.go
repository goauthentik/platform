package device

import (
	"context"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedNSSServer

	api *api.APIClient
	log *log.Entry

	ctx    context.Context
	cancel context.CancelFunc
}

func NewServer() (component.Component, error) {
	ac, err := config.Get().Domains()[0].APIClient()
	if err != nil {
		return nil, err
	}
	srv := &Server{
		api: ac,
		log: systemlog.Get().WithField("logger", "sysd.device"),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	return srv, nil
}

func (ds *Server) Start() {}

func (ds *Server) Stop() error {
	ds.cancel()
	return nil
}

func (ds *Server) Register(grpc.ServiceRegistrar) {}
