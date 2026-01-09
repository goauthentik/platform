package config

import (
	"errors"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

var (
	ErrProfileNotFound = errors.New("profile not found")
)

type ConfigV1 struct {
	Debug    bool                        `json:"debug"`
	Profiles map[string]*ConfigV1Profile `json:"profiles"`

	log *log.Entry
}

func (c ConfigV1) Default() cfgmgr.Configer {
	return ConfigV1{
		Debug:    false,
		Profiles: map[string]*ConfigV1Profile{},
		log:      log.WithField("logger", "storage.config"),
	}
}

func (c ConfigV1) PostUpdate(prev cfgmgr.Configer, evt fsnotify.Event) cfgmgr.ConfigChangedType {
	previousConfig := prev.(ConfigV1)
	if len(previousConfig.Profiles) < len(c.Profiles) {
		return cfgmgr.ConfigChangedAdded
	} else if len(previousConfig.Profiles) > len(c.Profiles) {
		return cfgmgr.ConfigChangedRemoved
	}
	return cfgmgr.ConfigChangedGeneric
}

type ConfigV1Profile struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`

	// Not saved to JSON, loaded from keychain
	AccessToken  string `json:"-"`
	RefreshToken string `json:"-"`

	// Fallback if keyring isn't available
	FallbackAccessToken  string `json:"access_token"`
	FallbackRefreshToken string `json:"refresh_token"`
}
