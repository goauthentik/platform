package config

import (
	"encoding/json"
	"os"
	"path/filepath"

	"github.com/fsnotify/fsnotify"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/platform/keyring"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
	"goauthentik.io/platform/pkg/storage/state"
)

var manager *cfgmgr.Manager[*Config]
var st *state.State

func Init(path string) error {
	sst, err := state.Open(types.StatePath().ForCurrent(), nil)
	if err != nil {
		return err
	}
	st = sst
	m, err := cfgmgr.NewManager[*Config](path)
	if err != nil {
		return err
	}
	manager = m
	return nil
}

func Manager() *cfgmgr.Manager[*Config] {
	return manager
}

func State() *state.State {
	return st
}

type Config struct {
	Debug      bool   `json:"debug"`
	RuntimeDir string `json:"runtime"`
	DomainDir  string `json:"domains"`

	log     *log.Entry
	domains []*DomainConfig
}

func (c *Config) Default() cfgmgr.Configer {
	return &Config{
		log: log.WithField("logger", "storage.config"),
	}
}

func (c *Config) PostLoad() error {
	err := c.loadDomains()
	if err != nil {
		c.log.WithError(err).Warning("failed to load domains")
	}
	return nil
}

func (c *Config) PreSave() error { return nil }
func (c *Config) PostUpdate(cfgmgr.Configer, fsnotify.Event) cfgmgr.ConfigChangedType {
	return cfgmgr.ConfigChangedGeneric
}

func (c *Config) Domains() []*DomainConfig {
	return c.domains
}

func (c *Config) SaveDomain(dom *DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".json")
	err := keyring.Set(keyring.Service("domain_token"), dom.Domain, keyring.AccessibleAlways, dom.Token)
	if err != nil {
		if !errors.Is(err, keyring.ErrUnsupportedPlatform) {
			c.log.WithError(err).Warning("failed to save domain token in keyring")
		}
		dom.FallbackToken = dom.Token
	}
	b, err := json.Marshal(dom)
	if err != nil {
		return err
	}
	err = os.WriteFile(path, b, 0o700)
	if err != nil {
		return errors.Wrap(err, "failed to save domain config")
	}
	return nil
}
