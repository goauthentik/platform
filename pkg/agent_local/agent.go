package agentlocal

import (
	"os"
	"os/signal"
	"syscall"

	"github.com/nightlyone/lockfile"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/agent_local/tray"
	"goauthentik.io/platform/pkg/agent_local/tray/available"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
	"google.golang.org/grpc"
)

type Agent struct {
	pb.UnimplementedAgentAuthServer
	pb.UnimplementedAgentCacheServer
	pb.UnimplementedAgentCtrlServer
	pb.UnimplementedPingServer

	grpc *grpc.Server
	cfg  *cfgmgr.Manager[config.ConfigV1]
	tr   *token.GlobalTokenManager
	log  *log.Entry
	tray *tray.Tray
	lock lockfile.Lockfile
	lis  socket.InfoListener
	bus  *events.Bus
}

func New() (*Agent, error) {
	mgr := config.Manager()
	l := systemlog.Get().WithField("logger", "agent")
	b := events.New(l)
	config.Manager().SetBus(b)
	return &Agent{
		cfg:  mgr,
		log:  l,
		tr:   token.NewGlobal(),
		tray: tray.New(mgr),
		bus:  b,
	}, nil
}

func (a *Agent) Start() {
	if !available.SystrayAvailable() {
		a.StartForeground()
		return
	}
	err := a.AcquireLock()
	if err != nil {
		a.log.Error("failed to acquire Lock. Authentik agent is already running.")
		os.Exit(1)
		return
	}
	go a.startGRPC()
	go a.signalHandler()
	go func() {
		<-a.tray.Exit
		a.Stop()
	}()
	a.tray.Start()
}

func (a *Agent) StartForeground() {
	err := a.AcquireLock()
	if err != nil {
		a.log.Error("failed to acquire Lock. Authentik agent is already running.")
		os.Exit(1)
		return
	}
	go a.startGRPC()
	a.signalHandler()
}

func (a *Agent) signalHandler() {
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	<-sigChan

	log.Info("Shutting down...")
	if available.SystrayAvailable() {
		a.tray.Quit()
	}
}

func (a *Agent) Stop() {
	a.log.WithField("lock", a.lock).Info("Removing lock file")
	_ = a.lock.Unlock()
	if a.grpc != nil {
		a.grpc.Stop()
	}
}
