package agentsystem

import (
	"net"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/storage"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

var agentCmd = &cobra.Command{
	Use:          "agent",
	Short:        "Run the authentik system agent",
	SilenceUsage: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		err := agentPrecheck()
		if err != nil {
			return err
		}
		if _, err := os.Stat(config.Manager().Get().RuntimeDir()); err != nil {
			return errors.Wrap(err, "failed to check runtime directory")
		}
		return nil
	},
	Run: func(cmd *cobra.Command, args []string) {
		log.SetLevel(log.DebugLevel)
		New().Start()
	},
}

func init() {
	rootCmd.AddCommand(agentCmd)
}

type SystemAgent struct {
	log *log.Entry
	srv *grpc.Server
	cm  map[string]component.Component
	mtx sync.Mutex
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
		cm:  map[string]component.Component{},
		mtx: sync.Mutex{},
	}
	sm.DomainCheck()

	sm.mtx.Lock()
	for name, constr := range sm.RegisterPlatformComponents() {
		comp, err := constr()
		if err != nil {
			panic(err)
		}
		sm.cm[name] = comp
		comp.Register(sm.srv)
	}
	sm.mtx.Unlock()
	go sm.watchConfig()
	return sm
}

func (sm *SystemAgent) DomainCheck() {
	for _, dom := range config.Manager().Get().Domains() {
		err := dom.Test()
		if err != nil {
			sm.log.WithField("domain", dom.Domain).WithError(err).Warning("failed to get API client for domain")
			dom.Enabled = false
		}
	}
}

func (sm *SystemAgent) watchConfig() {
	sm.log.Debug("Starting config file watch")
	for evt := range config.Manager().Watch() {
		sm.log.WithField("evt", evt).Debug("Handling config event")
		if evt.Type == storage.ConfigChangedAdded || evt.Type == storage.ConfigChangedRemoved {
			sm.mtx.Lock()
			for n, component := range sm.cm {
				err := component.Stop()
				if err != nil {
					sm.log.WithError(err).WithField("component", n).Warning("failed to stop componnet")
					sm.mtx.Unlock()
					continue
				}
				component.Start()
			}
			sm.mtx.Unlock()
		}
	}
}

func (sm *SystemAgent) Start() {
	sm.mtx.Lock()
	for _, component := range sm.cm {
		component.Start()
	}
	sm.mtx.Unlock()

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		sm.log.Info("Shutting down...")

		sm.mtx.Lock()
		defer sm.mtx.Unlock()
		for n, comp := range sm.cm {
			err := comp.Stop()
			if err != nil {
				sm.log.WithError(err).WithField("component", n).Warning("failed to stop component")
			}
		}
		sm.srv.GracefulStop()
		_ = os.Remove(config.Manager().Get().Socket)
	}()

	_ = os.Remove(config.Manager().Get().Socket)
	lis, err := net.Listen("unix", config.Manager().Get().Socket)
	if err != nil {
		sm.log.WithError(err).Fatal("Failed to listen")
	}
	_ = os.Chmod(config.Manager().Get().Socket, 0666)

	sm.log.WithField("path", config.Manager().Get().Socket).Info("System agent listening on socket")
	if err := sm.srv.Serve(lis); err != nil {
		sm.log.WithError(err).Fatal("Failed to serve")
	}
}
