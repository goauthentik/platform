package agent

import (
	"time"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/cfg"
)

type Agent struct {
	cfg            *cfg.ConfigManager
	tr             *ak.TokenRefresher
	log            *log.Entry
	systrayStarted bool
}

func New() (*Agent, error) {
	mgr, err := cfg.Manager()
	if err != nil {
		return nil, err
	}
	return &Agent{
		cfg: mgr,
		log: log.WithField("logger", "agent"),
		tr:  ak.NewTokenRefresher(mgr, mgr.Get().Profiles["default"]),
	}, nil
}

func (a *Agent) Start() {
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
	ch, err := a.cfg.Watch()
	if err != nil {
		a.log.WithError(err).Warning("failed to watch config")
		time.Sleep(5 * time.Second)
		a.startConfigWatch()
		return
	}
	for evt := range ch {
		if evt.Has(fsnotify.Write) {
			a.log.Debug("config file changed, triggering config reload")
			err = a.cfg.Load()
			if err != nil {
				a.log.WithError(err).Warning("failed to reload config")
				continue
			}
			a.tokenWatch()
			a.systrayConfigUpdate()
		}
	}
}
