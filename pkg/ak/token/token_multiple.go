package token

import (
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

var globalMutex = false

type GlobalTokenManager struct {
	log      *log.Entry
	managers map[string]*ProfileTokenManager
	mlock    sync.RWMutex
}

func NewGlobal() *GlobalTokenManager {
	if globalMutex {
		panic("Only a single global token manager can be used")
	}
	globalMutex = true
	gtm := &GlobalTokenManager{
		log:      systemlog.Get().WithField("logger", "token.manager.global"),
		managers: make(map[string]*ProfileTokenManager, 0),
		mlock:    sync.RWMutex{},
	}
	gtm.start()
	return gtm
}

func (gtm *GlobalTokenManager) start() {
	gtm.mlock.Lock()
	for n := range config.Manager().Get().Profiles {
		m, err := NewProfileVerified(n)
		if err != nil {
			gtm.log.WithError(err).WithField("profile", n).Warning("failed to create manager for profile")
			continue
		}
		gtm.managers[n] = m
	}
	gtm.mlock.Unlock()
	go func() {
		for evt := range config.Manager().Watch() {
			gtm.eventHandler(evt)
		}
	}()
}

func delta(a config.ConfigV1, b config.ConfigV1) []string {
	delta := []string{}
	for ap := range a.Profiles {
		found := false
		for bp := range b.Profiles {
			if bp == ap {
				found = true
			}
		}
		if !found {
			delta = append(delta, ap)
		}
	}
	return delta
}

func (gtm *GlobalTokenManager) eventHandler(evt cfgmgr.ConfigChangedEvent[config.ConfigV1]) {
	gtm.mlock.Lock()
	defer gtm.mlock.Unlock()
	if evt.Type == cfgmgr.ConfigChangedAdded {
		d := delta(evt.PreviousConfig, config.Manager().Get())
		for _, dd := range d {
			m, err := NewProfileVerified(dd)
			if err != nil {
				gtm.log.WithError(err).WithField("profile", dd).Warning("failed to create manager for profile")
				continue
			}
			gtm.managers[dd] = m
		}
	} else if evt.Type == cfgmgr.ConfigChangedRemoved {
		d := delta(config.Manager().Get(), evt.PreviousConfig)
		for _, dd := range d {
			mgr := gtm.managers[dd]
			mgr.Stop()
			delete(gtm.managers, dd)
		}
	}
}

func (gtm *GlobalTokenManager) ForProfile(name string) *ProfileTokenManager {
	gtm.mlock.RLock()
	defer gtm.mlock.RUnlock()
	return gtm.managers[name]
}
