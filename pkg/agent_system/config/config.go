package config

import (
	"os"
	"path"
	"path/filepath"
	"strings"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"

	"gopkg.in/yaml.v3"
)

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

	path    string
	log     *log.Entry
	domains *[]DomainConfig
}

func (c *Config) RuntimeDir() string {
	return path.Join("/var/run", "authentik")
}

func (c *Config) Domains() []DomainConfig {
	if c.domains != nil {
		return *c.domains
	}
	m, err := filepath.Glob(filepath.Join(c.DomainDir, "**.yml"))
	if err != nil {
		c.log.WithError(err).Warning("failed to load domains")
		return []DomainConfig{}
	}
	dom := []DomainConfig{}
	for _, match := range m {
		co, err := os.ReadFile(match)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		d := DomainConfig{}
		err = yaml.Unmarshal(co, &d)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		c.log.WithField("domain", d.Domain).Info("loaded domain")
		dom = append(dom, d)
	}
	c.domains = &dom
	return *c.domains
}

func (c *Config) SaveDomain(dom DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".yml")
	b, err := yaml.Marshal(dom)
	if err != nil {
		return err
	}
	return os.WriteFile(path, b, 0o700)
}

func (c *Config) Watch() error {
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return err
	}
	reloadConfig := func() {
		c.log.Debug("config file changed, triggering config reload")
		err = c.Load()
		if err != nil {
			c.log.WithError(err).Warning("failed to reload config")
			return
		}
	}
	go func() {
		for {
			select {
			case event, ok := <-watcher.Events:
				if !ok {
					continue
				}
				if event.Name != c.path || !strings.HasPrefix(event.Name, c.DomainDir) {
					continue
				}
				if event.Has(fsnotify.Write) {
					continue
				}
				c.log.WithField("event", event).Debug("config file update")
				Load(c.path)
				c.domains = nil
				for _, ch := range c.changed {
					ch <- evt
				}
			case err, ok := <-watcher.Errors:
				if !ok {
					continue
				}
				c.log.WithError(err).Warning("error watching file")
			}
		}
	}()

	err = watcher.Add(path.Dir(c.path))
	if err != nil {
		return err
	}
	return nil
}
