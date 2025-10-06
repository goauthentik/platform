package config

import (
	"github.com/adrg/xdg"
	"goauthentik.io/cli/pkg/storage"
)

var manager *storage.ConfigManager[ConfigV1]

func Manager() *storage.ConfigManager[ConfigV1] {
	if manager == nil {
		file, err := xdg.ConfigFile("authentik/config.json")
		if err != nil {
			panic(err)
		}
		m, err := storage.NewManager[ConfigV1](file)
		if err != nil {
			panic(err)
		}
		manager = m
	}
	return manager
}
