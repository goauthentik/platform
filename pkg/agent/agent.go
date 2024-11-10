package agent

import (
	"os"

	"github.com/nightlyone/lockfile"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/cfg"
)

type Agent struct {
	cfg            *cfg.ConfigManager
	tr             *ak.TokenRefresher
	log            *log.Entry
	systrayStarted bool
	lock           lockfile.Lockfile
}

func New() (*Agent, error) {
	mgr := cfg.Manager()
	return &Agent{
		cfg: mgr,
		log: log.WithField("logger", "agent"),
		tr:  ak.NewTokenRefresher(mgr),
	}, nil
}

func (a *Agent) Start() {
	err := a.AcquireLock()
	if err != nil {
		a.log.Error("failed to acquire Lock. Authentik agent is already running.")
		os.Exit(1)
		return
	}
	a.tokenWatch()
	go a.startConfigWatch()
	a.startSystray()
}

func (a *Agent) tokenWatch() {
	// Ensure the access token is not expired
	for profileName := range a.cfg.Get().Profiles {
		a.log.WithField("profile", profileName).Debug("checking if access/refresh token needs to be refreshed")
		a.tr.AccessToken(profileName)
	}
}

func (a *Agent) startConfigWatch() {
	a.log.Debug("Starting config file watch")
	for range a.cfg.Watch() {
		a.tokenWatch()
		a.systrayConfigUpdate()
	}
}
