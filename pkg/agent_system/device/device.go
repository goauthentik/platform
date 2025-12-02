package device

import (
	"context"
	"errors"
	"time"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "device"

type Server struct {
	pb.UnimplementedAgentPlatformServer

	dom *config.DomainConfig
	api *api.APIClient
	log *log.Entry

	ctx context.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
		ctx: ctx.Context(),
	}
	return srv, nil
}

func (ds *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	dom := config.Manager().Get().Domains()[0]
	ac, err := dom.APIClient()
	if err != nil {
		return err
	}
	ds.dom = dom
	ds.api = ac
	go ds.checkIn()
	d := time.Second * time.Duration(dom.Config().RefreshInterval)
	t := time.NewTicker(d)
	go func() {
		for {
			select {
			case <-t.C:
				ds.log.Info("Starting checkin")
				ds.checkIn()
				ds.log.WithField("next", d.String()).Info("Finished checkin")
			case <-ds.ctx.Done():
				return
			}
		}
	}()
	return nil
}

func (ds *Server) Stop() error {
	return nil
}

func (ds *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterAgentPlatformServer(s, ds)
}
