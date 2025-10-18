package agentsystem

import (
	"context"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/storage"
	"google.golang.org/grpc"
)

type ComponentInstance struct {
	comp   component.Component
	ctx    context.Context
	cancel context.CancelFunc
}

type SystemAgent struct {
	log    *log.Entry
	srv    *grpc.Server
	cm     map[string]ComponentInstance
	mtx    sync.Mutex
	ctx    context.Context
	cancel context.CancelFunc
	lis    socket.InfoListener
}

func New() *SystemAgent {
	l := systemlog.Get().WithField("logger", "sysd")

	sm := &SystemAgent{
		srv: grpc.NewServer(
			grpc.ChainUnaryInterceptor(
				logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l)),
				grpc_sentry.UnaryServerInterceptor(),
			),
			grpc.ChainStreamInterceptor(
				logging.StreamServerInterceptor(systemlog.InterceptorLogger(l)),
				grpc_sentry.StreamServerInterceptor(),
			),
		),
		log: l,
		cm:  map[string]ComponentInstance{},
		mtx: sync.Mutex{},
	}
	sm.ctx, sm.cancel = context.WithCancel(context.Background())
	sm.DomainCheck()
	sm.registerComponents()

	go sm.watchConfig()
	return sm
}

func (sm *SystemAgent) registerComponents() {
	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	for name, constr := range sm.RegisterPlatformComponents() {
		l := sm.log.WithField("component", name)
		l.Info("Registering component")
		ctx, cancel := context.WithCancel(sm.ctx)
		comp, err := constr(component.Context{
			Log:     l,
			Context: ctx,
		})
		if err != nil {
			panic(err)
		}
		sm.cm[name] = ComponentInstance{
			comp:   comp,
			ctx:    ctx,
			cancel: cancel,
		}
		comp.Register(sm.srv)
	}
}

func (sm *SystemAgent) DomainCheck() {
	for _, dom := range config.Manager().Get().Domains() {
		err := dom.Test()
		if err != nil {
			sm.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to get API client for domain")
			dom.Enabled = false
			continue
		}
		sm.log.WithField("domain", dom.Domain).Info("Tested domain connectivity")
	}
}

func (sm *SystemAgent) watchConfig() {
	sm.log.Debug("Starting config file watch")
	for evt := range config.Manager().Watch() {
		sm.log.WithField("evt", evt).Debug("Handling config event")
		if evt.Type == storage.ConfigChangedAdded || evt.Type == storage.ConfigChangedRemoved {
			sm.mtx.Lock()
			for n, component := range sm.cm {
				err := component.comp.Stop()
				if err != nil {
					sm.log.WithError(err).WithField("component", n).Warning("failed to stop componnet")
					continue
				}
				err = component.comp.Start()
				if err != nil {
					sm.log.WithError(err).WithField("component", n).Info("Failed to start component")
					continue
				}
			}
			sm.mtx.Unlock()
		}
	}
}

func (sm *SystemAgent) Start() {
	sm.mtx.Lock()
	for n, component := range sm.cm {
		sm.log.WithField("component", n).Info("Starting component")
		err := component.comp.Start()
		if err != nil {
			sm.log.WithError(err).WithField("component", n).Info("Failed to start component")
			continue
		}
	}
	sm.mtx.Unlock()

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		sm.Stop()
	}()

	lis, err := socket.Listen(types.GetSysdSocketPath(), socket.SocketEveryone)
	if err != nil {
		sm.log.WithError(err).Fatal("Failed to listen")
		return
	}
	sm.lis = lis

	sm.log.WithField("path", lis.Path().ForCurrent()).Info("System agent listening on socket")
	if err := sm.srv.Serve(lis); err != nil {
		sm.log.WithError(err).Fatal("Failed to serve")
	}
}

func (sm *SystemAgent) Stop() {
	sm.log.Info("Shutting down...")

	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	for n, comp := range sm.cm {
		err := comp.comp.Stop()
		if err != nil {
			sm.log.WithError(err).WithField("component", n).Warning("failed to stop component")
		}
	}
	sm.srv.GracefulStop()
	if sm.lis != nil {
		err := sm.lis.Close()
		if err != nil {
			sm.log.WithError(err).Warning("failed to stop listening")
		}
	}
}
