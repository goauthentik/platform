package agentlocal

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	"github.com/kolide/systray"
	"github.com/nightlyone/lockfile"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
	"google.golang.org/grpc"
)

type Agent struct {
	pb.UnimplementedAgentAuthServer
	pb.UnimplementedAgentCacheServer
	pb.UnimplementedAgentConfigServer

	grpc           *grpc.Server
	cfg            *cfgmgr.Manager[config.ConfigV1]
	tr             *token.GlobalTokenManager
	log            *log.Entry
	systrayStarted bool
	lock           lockfile.Lockfile
	systrayCtx     context.Context
	systrayCtxS    context.CancelFunc
	lis            socket.InfoListener
}

func New() (*Agent, error) {
	mgr := config.Manager()
	return &Agent{
		cfg: mgr,
		log: systemlog.Get().WithField("logger", "agent"),
		tr:  token.NewGlobal(),
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
	if a.lis != nil {
		err := a.lis.Close()
		if err != nil {
			a.log.WithError(err).Warning("failed to close socket")
		}
	}
}

func (a *Agent) startConfigWatch() {
	a.log.Debug("Starting config file watch")
	for range a.cfg.Watch() {
		a.systrayConfigUpdate()
	}
}
