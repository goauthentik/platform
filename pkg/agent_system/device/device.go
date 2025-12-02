package device

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "device"

type Server struct {
	pb.UnimplementedAgentPlatformServer

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
	for _, dom := range config.Manager().Get().Domains() {
		go ds.checkIn(dom)
		d := time.Second * time.Duration(dom.Config().RefreshInterval)
		t := time.NewTicker(d)
		go func() {
			for {
				select {
				case <-t.C:
					ds.log.WithField("domain", dom.Domain).Info("Starting checkin")
					ds.checkIn(dom)
					ds.log.WithField("domain", dom.Domain).WithField("next", d.String()).Info("Finished checkin")
				case <-ds.ctx.Done():
					return
				}
			}
		}()
	}
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
