package device

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	ctypes "goauthentik.io/platform/pkg/agent_system/ctrl/types"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/events"
	"google.golang.org/grpc"
)

const ID = "device"

type Server struct {
	pb.UnimplementedAgentPlatformServer

	log *log.Entry

	ctx component.Context

	cancel context.CancelFunc
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
		ctx: ctx,
	}
	return srv, nil
}

func (ds *Server) Start() error {
	ds.ctx.Bus().AddEventListener(ctypes.TopicCtrlDomainChanged, func(ev *events.Event) {
		if ds.cancel != nil {
			ds.cancel()
		}
		ds.runCheckins()
	})
	ds.runCheckins()
	return nil
}

func (ds *Server) runCheckins() {
	ctx, cancel := context.WithCancel(ds.ctx.Context())
	ds.cancel = cancel
	for _, dom := range config.Manager().Get().Domains() {
		go ds.checkIn(ctx, dom)
		d := time.Second * time.Duration(dom.Config().RefreshInterval)
		t := time.NewTicker(d)
		go func() {
			for {
				select {
				case <-t.C:
					ds.log.WithField("domain", dom.Domain).Info("Starting checkin")
					ds.checkIn(ctx, dom)
					ds.log.WithField("domain", dom.Domain).WithField("next", d.String()).Info("Finished checkin")
				case <-ctx.Done():
					return
				}
			}
		}()
	}

}

func (ds *Server) Stop() error {
	if ds.cancel != nil {
		ds.cancel()
	}
	return nil
}

func (ds *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterAgentPlatformServer(s, ds)
}
