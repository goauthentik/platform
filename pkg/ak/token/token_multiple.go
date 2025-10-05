package token

import (
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent_local/storage"
	gstorage "goauthentik.io/cli/pkg/storage"
	"goauthentik.io/cli/pkg/systemlog"
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
	for n := range storage.Manager().Get().Profiles {
		m, err := NewProfileVerified(n)
		if err != nil {
			gtm.log.WithError(err).WithField("profile", n).Warning("failed to create manager for profile")
			continue
		}
		gtm.managers[n] = m
	}
	gtm.mlock.Unlock()
	go func() {
		for evt := range storage.Manager().Watch() {
			gtm.eventHandler(evt)
		}
	}()
}

func delta(a storage.ConfigV1, b storage.ConfigV1) []string {
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

func (gtm *GlobalTokenManager) eventHandler(evt gstorage.ConfigChangedEvent[storage.ConfigV1]) {
	gtm.mlock.Lock()
	defer gtm.mlock.Unlock()
	if evt.Type == gstorage.ConfigChangedAdded {
		d := delta(evt.PreviousConfig, storage.Manager().Get())
		for _, dd := range d {
			m, err := NewProfileVerified(dd)
			if err != nil {
				gtm.log.WithError(err).WithField("profile", dd).Warning("failed to create manager for profile")
				continue
			}
			gtm.managers[dd] = m
		}
	} else if evt.Type == gstorage.ConfigChangedRemoved {
		d := delta(storage.Manager().Get(), evt.PreviousConfig)
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
