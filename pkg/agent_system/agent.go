package agentsystem

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"slices"
	"sync"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/recovery"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
	"goauthentik.io/platform/pkg/storage/state"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type ComponentInstance struct {
	comp   component.Component
	ctx    context.Context
	cancel context.CancelFunc
	st     *state.ScopedState
}

type SocketServer struct {
	ID     string
	Path   pstr.PlatformString
	Server *grpc.Server
	Perm   socket.SocketPermMode
}

type SystemAgent struct {
	log *log.Entry

	srv []SocketServer

	cm     map[string]ComponentInstance
	mtx    sync.Mutex
	ctx    context.Context
	cancel context.CancelFunc
	st     *state.State
	opts   SystemAgentOptions
	b      *events.Bus
}

type SystemAgentOptions struct {
	DisabledComponents []string
}

func New(opts SystemAgentOptions) (*SystemAgent, error) {
	l := systemlog.Get().WithField("logger", "sysd")
	serverOpts := []grpc.ServerOption{
		grpc.ChainUnaryInterceptor(
			logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.UnaryServerInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			}), grpc_sentry.WithRepanicOption(true)),
			recovery.UnaryServerInterceptor(recovery.WithRecoveryHandler(func(p any) (err error) {
				l.WithField("p", p).Warning("GRPC method panicd")
				return status.Errorf(codes.Unknown, "panic triggered")
			})),
		),
		grpc.ChainStreamInterceptor(
			logging.StreamServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.StreamServerInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			}), grpc_sentry.WithRepanicOption(true)),
			recovery.StreamServerInterceptor(recovery.WithRecoveryHandler(func(p any) (err error) {
				l.WithField("p", p).Warning("GRPC method panicd")
				return status.Errorf(codes.Unknown, "panic triggered")
			})),
		),
	}
	sm := &SystemAgent{
		srv: []SocketServer{
			{
				ID:     types.SocketIDDefault,
				Server: grpc.NewServer(serverOpts...),
				Path:   types.GetSysdSocketPath(types.SocketIDDefault),
				Perm:   socket.SocketEveryone,
			},
			{
				ID:     types.SocketIDCtrl,
				Server: grpc.NewServer(serverOpts...),
				Path:   types.GetSysdSocketPath(types.SocketIDCtrl),
				Perm:   socket.SocketAdmin,
			},
		},
		log:  l,
		cm:   map[string]ComponentInstance{},
		mtx:  sync.Mutex{},
		opts: opts,
		st:   config.State(),
		b:    events.New(l),
	}
	config.Manager().SetBus(sm.b.Child("config"))
	sm.ctx, sm.cancel = context.WithCancel(context.Background())
	sm.registerComponents()
	sm.DomainCheck()

	sm.b.AddEventListener(cfgmgr.TopicConfigChanged, func(ev *events.Event) {
		sm.log.WithField("ev", ev).Debug("Handling config event")
		et := ev.Payload.Data["type"].(cfgmgr.ConfigChangedType)
		if et == cfgmgr.ConfigChangedAdded || et == cfgmgr.ConfigChangedRemoved {
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
	})
	return sm, nil
}

func (sm *SystemAgent) registerComponents() {
	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	for name, constr := range sm.RegisterPlatformComponents() {
		l := sm.log.WithField("logger", fmt.Sprintf("component.%s", name))
		if slices.Contains(sm.opts.DisabledComponents, name) {
			l.Info("Component disabled")
			continue
		}
		l.Info("Registering component")
		ctx, cancel := context.WithCancel(sm.ctx)
		ss := sm.st.ForBucket(types.KeyComponent, name)
		cctx := component.NewContext(ctx, l, sm, ss, sm.b.Child(name))
		comp, err := constr(cctx)
		if err != nil {
			panic(err)
		}
		err = ss.EnsureBucket()
		if err != nil {
			l.WithError(err).Warning("failed to ensure bucket for component")
			cancel()
			continue
		}
		sm.cm[name] = ComponentInstance{
			comp:   comp,
			ctx:    ctx,
			cancel: cancel,
			st:     ss,
		}
		for _, srv := range sm.srv {
			comp.RegisterForID(srv.ID, srv.Server)
		}
	}
}

func (sm *SystemAgent) GetComponent(id string) component.Component {
	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	ci, ok := sm.cm[id]
	if !ok {
		return nil
	}
	return ci.comp
}

func (sm *SystemAgent) DomainCheck() {
	sm.log.Info("Starting domain healthcheck")
	for _, dom := range config.Manager().Get().Domains() {
		sm.log.WithField("domain", dom.Domain).Info("Starting domain healthcheck for domain")
		err := dom.Test()
		if err != nil {
			sm.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to test API client for domain")
			dom.Enabled = false
			continue
		}
		sm.log.WithField("domain", dom.Domain).Info("Tested domain connectivity")
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

	for _, srv := range sm.srv {
		go func(srv SocketServer) {
			l := sm.log.WithField("srv", srv.ID)
			lis, err := socket.Listen(srv.Path, srv.Perm)
			if err != nil {
				l.WithError(err).Fatal("Failed to listen")
				return
			}

			l.WithField("path", lis.Path().ForCurrent()).Info("System agent listening on socket")
			if err := srv.Server.Serve(lis); err != nil {
				l.WithError(err).Fatal("Failed to serve")
			}
		}(srv)
	}

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	<-sigChan

	sm.Stop()
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
	err := sm.st.Close()
	if err != nil {
		sm.log.WithError(err).Warning("failed to close state")
	}
	for _, srv := range sm.srv {
		srv.Server.GracefulStop()
	}
}
