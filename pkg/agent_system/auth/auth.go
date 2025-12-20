package auth

import (
	"errors"
	"runtime"
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "auth"

type Server struct {
	pb.UnimplementedSystemAuthTokenServer
	pb.UnimplementedSystemAuthInteractiveServer
	pb.UnimplementedSystemAuthAuthorizeServer
	pb.UnimplementedSystemAuthAppleServer

	api *api.APIClient
	log *log.Entry

	ctx component.Context

	m   sync.RWMutex
	dom *config.DomainConfig

	interactiveEnabled   bool
	authorizationEnabled bool
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log:                  ctx.Log(),
		ctx:                  ctx,
		m:                    sync.RWMutex{},
		interactiveEnabled:   true,
		authorizationEnabled: true,
	}
	return srv, nil
}

func NewTokenServer(ctx component.Context) (component.Component, error) {
	srv, err := NewServer(ctx)
	if err != nil {
		return nil, err
	}
	srv.(*Server).interactiveEnabled = false
	srv.(*Server).authorizationEnabled = false
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
	return nil
}

func (auth *Server) Stop() error {
	return nil
}

func (auth *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterSystemAuthTokenServer(s, auth)
	if auth.interactiveEnabled {
		pb.RegisterSystemAuthInteractiveServer(s, auth)
	}
	if auth.authorizationEnabled {
		pb.RegisterSystemAuthAuthorizeServer(s, auth)
	}
	if runtime.GOOS == "darwin" {
		pb.RegisterSystemAuthAppleServer(s, auth)
	}
}
