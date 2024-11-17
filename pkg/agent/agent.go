package agent

import (
	"context"
	"os"

	"github.com/nightlyone/lockfile"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
)

type Agent struct {
	cfg            *storage.ConfigManager
	tr             *token.GlobalTokenManager
	log            *log.Entry
	systrayStarted bool
	lock           lockfile.Lockfile
	systrayCtx     context.Context
	systrayCtxS    context.CancelFunc
}

func New() (*Agent, error) {
	mgr := storage.Manager()
	return &Agent{
		cfg: mgr,
		log: log.WithField("logger", "agent"),
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
	a.startSystray()
}

func (a *Agent) startConfigWatch() {
	a.log.Debug("Starting config file watch")
	for range a.cfg.Watch() {
		a.systrayConfigUpdate()
	}
}
