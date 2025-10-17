package config

import (
	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/storage"
)

type ConfigV1 struct {
	Debug    bool                        `json:"debug"`
	Profiles map[string]*ConfigV1Profile `json:"profiles"`

	log *log.Entry
}

func (c ConfigV1) Default() storage.Configer {
	return ConfigV1{
		Debug:    false,
		Profiles: map[string]*ConfigV1Profile{},
		log:      log.WithField("logger", "storage.config"),
	}
}

func (c ConfigV1) PostUpdate(prev storage.Configer, evt fsnotify.Event) storage.ConfigChangedType {
	previousConfig := prev.(ConfigV1)
	if len(previousConfig.Profiles) < len(c.Profiles) {
		return storage.ConfigChangedAdded
	} else if len(previousConfig.Profiles) > len(c.Profiles) {
		return storage.ConfigChangedRemoved
	}
	return storage.ConfigChangedGeneric
}

type ConfigV1Profile struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`

	// Not saved to JSON, loaded from keychain
	AccessToken  string `json:"-"`
	RefreshToken string `json:"-"`
}
