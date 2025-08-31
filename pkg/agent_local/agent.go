package agentlocal

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	"github.com/kolide/systray"
	"github.com/nightlyone/lockfile"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent_local/types"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
	"google.golang.org/grpc"
)

type Agent struct {
	pb.UnimplementedAgentAuthServer
	pb.UnimplementedAgentCacheServer
	pb.UnimplementedAgentSetupServer

	grpc           *grpc.Server
	cfg            *storage.ConfigManager
	tr             *token.GlobalTokenManager
	log            *log.Entry
	systrayStarted bool
	lock           lockfile.Lockfile
	systrayCtx     context.Context
	systrayCtxS    context.CancelFunc
	socketPath     string
}

func New() (*Agent, error) {
	mgr := storage.Manager()
	return &Agent{
		cfg:        mgr,
		log:        log.WithField("logger", "agent"),
		tr:         token.NewGlobal(),
		socketPath: types.GetAgentSocketPath(),
	}, nil
}

func (a *Agent) Start() {
	err := a.AcquireLock()
	if err != nil {
		a.log.Error("failed to acquire Lock. Authentik agent is already running.")
		os.Exit(1)
		return
	}
	go a.startConfigWatch()
	go a.startGRPC()
	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		log.Info("Shutting down...")
		systray.Quit()
	}()
	a.startSystray()
}

func (a *Agent) Stop() {
	a.log.WithField("lock", a.lock).Info("Removing lock file")
	_ = a.lock.Unlock()
	if a.grpc != nil {
		a.grpc.Stop()
	}
	a.log.WithField("socket", a.socketPath).Info("Removing socket file")
	_ = os.Remove(a.socketPath)
}

func (a *Agent) startConfigWatch() {
	a.log.Debug("Starting config file watch")
	for range a.cfg.Watch() {
		a.systrayConfigUpdate()
	}
}
