package config

import (
	"strings"

	"github.com/adrg/xdg"
	"github.com/fsnotify/fsnotify"
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

var manager *cfgmgr.Manager[ConfigV1]

func Manager() *cfgmgr.Manager[ConfigV1] {
	if manager == nil {
		file, err := xdg.ConfigFile("authentik/config.json")
		if err != nil {
			panic(err)
		}
		m, err := cfgmgr.NewManager[ConfigV1](file)
		if err != nil {
			panic(err)
		}

		lock, err := xdg.ConfigFile("authentik/agent.lock")
		if err != nil {
			panic(err)
		}
		m.FilterWatchEvent = func(evt fsnotify.Event) bool {
			if evt.Name == lock {
				return false
			}
			if strings.HasPrefix(evt.Name, types.GetAgentSocketPath().ForCurrent()) {
				return false
			}
			return true
		}
		manager = m
	}
	return manager
}
