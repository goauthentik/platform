package config

import (
	"encoding/json"
	"os"
	"path"
	"path/filepath"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/storage"
)

var manager *storage.ConfigManager[*Config]

func Init(path string) error {
	m, err := storage.NewManager[*Config](path)
	if err != nil {
		return err
	}
	manager = m
	return nil
}

func Manager() *storage.ConfigManager[*Config] {
	return manager
}

type Config struct {
	Debug  bool   `yaml:"debug"`
	Socket string `yaml:"socket"`
	PAM    struct {
		Enabled           bool `yaml:"enabled"`
		TerminateOnExpiry bool `yaml:"terminate_on_expiry"`
	} `yaml:"pam" `
	NSS struct {
		Enabled            bool  `yaml:"enabled"`
		UIDOffset          int32 `yaml:"uid_offset"`
		GIDOffset          int32 `yaml:"gid_offset"`
		RefreshIntervalSec int64 `yaml:"refresh_interval_sec"`
	} `yaml:"nss"`
	DomainDir string `yaml:"domains"`

	log     *log.Entry
	domains []DomainConfig
}

func (c *Config) Default() storage.Configer {
	return &Config{
		log: log.WithField("logger", "storage.config"),
	}
}

func (c *Config) PostLoad() error {
	m, err := filepath.Glob(filepath.Join(c.DomainDir, "**.json"))
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
		c.log.WithField("domain", d.Domain).Info("loaded domain")
		dom = append(dom, d)
	}
	c.domains = dom
	return nil
}

func (c *Config) PreSave() error { return nil }
func (c *Config) PostUpdate(storage.Configer, fsnotify.Event) storage.ConfigChangedType {
	return storage.ConfigChangedGeneric
}

func (c *Config) RuntimeDir() string {
	return path.Join("/var/run", "authentik")
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
