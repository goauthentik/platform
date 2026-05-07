package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/fsnotify/fsnotify"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/platform/keyring"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
	"goauthentik.io/platform/pkg/storage/state"
)

// ensurePathWithinDir returns an error if fullPath resolves outside baseDir.
func ensurePathWithinDir(baseDir, fullPath string) error {
	absBase, err := filepath.Abs(baseDir)
	if err != nil {
		return err
	}
	absPath, err := filepath.Abs(fullPath)
	if err != nil {
		return err
	}
	if !strings.HasPrefix(absPath, absBase+string(filepath.Separator)) {
		return fmt.Errorf("path escapes domain directory")
	}
	return nil
}

var manager *cfgmgr.Manager[*Config]
var st *state.State

func Init(path string, statePath string) error {
	sst, err := state.Open(statePath, nil)
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
func (c *Config) PostUpdate(prev cfgmgr.Configer, evt fsnotify.Event) cfgmgr.ConfigChangedType {
	previousConfig := prev.(*Config)
	if len(previousConfig.domains) < len(c.domains) {
		return cfgmgr.ConfigChangedAdded
	} else if len(previousConfig.domains) > len(c.domains) {
		return cfgmgr.ConfigChangedRemoved
	}
	return cfgmgr.ConfigChangedGeneric
}

func (c *Config) Domains() []*DomainConfig {
	return c.domains
}

func (c *Config) SaveDomain(dom *DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".json")
	if err := ensurePathWithinDir(c.DomainDir, path); err != nil {
		return errors.Wrap(err, "invalid domain path")
	}
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
	err = os.WriteFile(path, b, 0o600)
	if err != nil {
		return errors.Wrap(err, "failed to save domain config")
	}
	return c.PostLoad()
}

func (c *Config) DeleteDomain(dom *DomainConfig) error {
	path := filepath.Join(c.DomainDir, dom.Domain+".json")
	if err := ensurePathWithinDir(c.DomainDir, path); err != nil {
		return errors.Wrap(err, "invalid domain path")
	}
	err := keyring.Delete(keyring.Service("domain_token"), dom.Domain, keyring.AccessibleAlways)
	if err != nil {
		if !errors.Is(err, keyring.ErrUnsupportedPlatform) {
			c.log.WithError(err).Warning("failed to delete domain token in keyring")
		}
		dom.FallbackToken = dom.Token
	}
	err = os.Remove(path)
	if err != nil {
		return errors.Wrap(err, "failed to delete domain config")
	}
	return c.PostLoad()
}
