package storage

import (
	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
)

type ConfigV1 struct {
	Debug    bool                        `json:"debug"`
	Profiles map[string]*ConfigV1Profile `json:"profiles"`

	log *log.Entry
}

func (c ConfigV1) Default() Configer {
	return ConfigV1{
		Debug:    false,
		Profiles: map[string]*ConfigV1Profile{},
		log:      log.WithField("logger", "storage.config"),
	}
}

func (c ConfigV1) PostUpdate(prev Configer, evt fsnotify.Event) ConfigChangedType {
	previousConfig := prev.(ConfigV1)
	if len(previousConfig.Profiles) < len(c.Profiles) {
		return ConfigChangedProfileAdded
	} else if len(previousConfig.Profiles) > len(c.Profiles) {
		return ConfigChangedProfileRemoved
	}
	return ConfigChangedGeneric
}

type ConfigV1Profile struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`

	// Not saved to JSON, loaded from keychain
	AccessToken  string `json:"-"`
	RefreshToken string `json:"-"`
}
