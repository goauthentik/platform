package config

import (
	"encoding/json"
	"os"
	"path/filepath"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
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
	Debug      bool   `json:"debug"`
	RuntimeDir string `json:"runtime"`
	DomainDir  string `json:"domains"`
	PAM        struct {
		Enabled           bool `json:"enabled"`
		TerminateOnExpiry bool `json:"terminate_on_expiry"`
	} `json:"pam" `
	NSS struct {
		Enabled            bool  `json:"enabled"`
		UIDOffset          int32 `json:"uid_offset"`
		GIDOffset          int32 `json:"gid_offset"`
		RefreshIntervalSec int64 `json:"refresh_interval_sec"`
	} `json:"nss"`

	log     *log.Entry
	domains []DomainConfig
}

func (c *Config) Default() cfgmgr.Configer {
	return &Config{
		log: log.WithField("logger", "storage.config"),
	}
}

func (c *Config) PostLoad() error {
	c.log.Debug("Loading domains...")
	m, err := filepath.Glob(filepath.Join(c.DomainDir, "*.json"))
	if err != nil {
		c.log.WithError(err).Warning("failed to load domains")
		return err
	}
	dom := []DomainConfig{}
	for _, match := range m {
		co, err := os.ReadFile(match)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		d := DomainConfig{}
		err = json.Unmarshal(co, &d)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		c.log.WithField("domain", d.Domain).Debug("loaded domain")
		dom = append(dom, d)
	}
	c.domains = dom
	return nil
}

func (c *Config) PreSave() error { return nil }
func (c *Config) PostUpdate(cfgmgr.Configer, fsnotify.Event) cfgmgr.ConfigChangedType {
	return cfgmgr.ConfigChangedGeneric
}

func (c *Config) Domains() []DomainConfig {
	return c.domains
}

func (c *Config) SaveDomain(dom DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".json")
	b, err := json.Marshal(dom)
	if err != nil {
		return err
	}
	return os.WriteFile(path, b, 0o700)
}
