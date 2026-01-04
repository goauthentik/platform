package token

import (
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/shared/events"
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
	config.Manager().Bus().AddEventListener(cfgmgr.TopicConfigChanged, func(ev *events.Event) {
		gtm.eventHandler(ev)
	})
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

func (gtm *GlobalTokenManager) eventHandler(ev *events.Event) {
	gtm.mlock.Lock()
	defer gtm.mlock.Unlock()
	typ := ev.Payload.Data["type"].(cfgmgr.ConfigChangedType)
	prev := ev.Payload.Data["previous_config"].(config.ConfigV1)
	if typ == cfgmgr.ConfigChangedAdded {
		d := delta(prev, config.Manager().Get())
		for _, dd := range d {
			m, err := NewProfileVerified(dd)
			if err != nil {
				gtm.log.WithError(err).WithField("profile", dd).Warning("failed to create manager for profile")
				continue
			}
			gtm.managers[dd] = m
		}
	} else if typ == cfgmgr.ConfigChangedRemoved {
		d := delta(config.Manager().Get(), prev)
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
