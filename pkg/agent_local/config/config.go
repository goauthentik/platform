package config

import (
	"github.com/adrg/xdg"
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
		manager = m
	}
	return manager
}
