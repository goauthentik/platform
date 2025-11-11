package auth

import (
	"errors"
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "auth"

type Server struct {
	pb.UnimplementedSystemAuthTokenServer
	pb.UnimplementedSystemAuthInteractiveServer
	pb.UnimplementedSystemAuthAuthorizeServer

	api *api.APIClient
	log *log.Entry

	ctx component.Context

	cfg  *config.Config
	txns map[string]*InteractiveAuthTransaction
	m    sync.RWMutex
	dom  config.DomainConfig
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log:  ctx.Log(),
		cfg:  config.Manager().Get(),
		ctx:  ctx,
		txns: map[string]*InteractiveAuthTransaction{},
		m:    sync.RWMutex{},
	}
	return srv, nil
}

func (auth *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	dom := config.Manager().Get().Domains()[0]
	ac, err := dom.APIClient()
	if err != nil {
		return err
	}
	auth.dom = dom
	auth.api = ac
	auth.startFetch()
	return nil
}

func (auth *Server) Stop() error {
	return nil
}

func (auth *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterSystemAuthAuthorizeServer(s, auth)
	pb.RegisterSystemAuthInteractiveServer(s, auth)
	pb.RegisterSystemAuthTokenServer(s, auth)
}
