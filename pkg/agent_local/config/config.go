package config

import (
	"slices"

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
		ignoredPaths := []string{
			lock,
			types.GetAgentSocketPath().ForCurrent(),
		}
		m.FilterWatchEvent = func(evt fsnotify.Event) bool {
			return !slices.Contains(ignoredPaths, evt.Name)
		}
		manager = m
	}
	return manager
}
