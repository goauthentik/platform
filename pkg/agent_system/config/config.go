package config

import (
	"encoding/json"
	"os"
	"path/filepath"

	"github.com/fsnotify/fsnotify"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/platform/keyring"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

var manager *cfgmgr.Manager[*Config]

func Init(path string) error {
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

type Config struct {
	Debug      bool      `json:"debug"`
	RuntimeDir string    `json:"runtime"`
	DomainDir  string    `json:"domains"`
	NSS        NSSConfig `json:"nss"`
	PAM        PAMConfig `json:"pam"`

	log     *log.Entry
	domains []DomainConfig
}

type PAMConfig struct {
	Enabled           bool `json:"enabled"`
	TerminateOnExpiry bool `json:"terminate_on_expiry"`
}

type NSSConfig struct {
	Enabled            bool  `json:"enabled"`
	UIDOffset          int32 `json:"uid_offset"`
	GIDOffset          int32 `json:"gid_offset"`
	RefreshIntervalSec int64 `json:"refresh_interval_sec"`
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

func (c *Config) Domains() []DomainConfig {
	return c.domains
}

func (c *Config) SaveDomain(dom *DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".json")
	err := keyring.Set(keyring.Service("domain_token"), dom.Domain, dom.Token)
	if err != nil {
		if !errors.Is(err, keyring.ErrUnsupportedPlatform) {
			return errors.Wrap(err, "failed to save domain token to keyring")
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
