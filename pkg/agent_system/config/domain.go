package config

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"time"

	"github.com/pkg/errors"
	"go.etcd.io/bbolt"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/platform/keyring"
)

type DomainConfig struct {
	Enabled      bool   `json:"enabled"`
	AuthentikURL string `json:"authentik_url"`
	Domain       string `json:"domain"`
	Managed      bool   `json:"managed"`

	// Saved to keyring
	Token string `json:"-"`
	// Fallback token when keyring is not available
	FallbackToken string `json:"token"`

	c  *api.APIClient
	r  *Config
	rc *api.AgentConfig
}

func (dc DomainConfig) Config() *api.AgentConfig {
	return dc.rc
}

func (dc DomainConfig) APIClient() (*api.APIClient, error) {
	if dc.c != nil {
		return dc.c, nil
	}
	u, err := url.Parse(dc.AuthentikURL)
	if err != nil {
		return nil, err
	}
	apiConfig := api.NewConfiguration()
	apiConfig.Host = u.Host
	apiConfig.Scheme = u.Scheme
	apiConfig.Servers = api.ServerConfigurations{
		{
			URL: fmt.Sprintf("%sapi/v3", u.Path),
		},
	}
	apiConfig.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer+agent %s", dc.Token))
	apiConfig.AddDefaultHeader("X-AK-Platform-Version", meta.Version)
	apiConfig.UserAgent = fmt.Sprintf("goauthentik.io/platform/%s", meta.FullVersion())

	c := api.NewAPIClient(apiConfig)
	dc.c = c
	return dc.c, nil
}

func (dc DomainConfig) Test() error {
	ac, err := dc.APIClient()
	if err != nil {
		return err
	}
	_, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAgentConfigRetrieve(context.Background()).Execute()
	if err != nil {
		if hr != nil && hr.StatusCode == http.StatusForbidden && dc.Managed {
			err := dc.Delete()
			if err != nil {
				dc.r.log.WithError(err).Warning("failed to delete domain")
				return err
			}
			return dc.r.loadDomainsManaged()
		}
		return err
	}
	return nil
}

func (dc DomainConfig) Delete() error {
	dp := filepath.Join(dc.r.DomainDir, dc.Domain+".json")
	err := os.Remove(dp)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	return nil
}

func (c *Config) NewDomain() *DomainConfig {
	return &DomainConfig{
		Enabled: true,
		r:       c,
	}
}

func (dc *DomainConfig) loaded() {
	err := State().ForBucket(dc.Domain).View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		cfg := api.NullableAgentConfig{}
		err := cfg.UnmarshalJSON(b.Get([]byte("config")))
		if err != nil {
			return err
		}
		dc.r.log.Info("Loaded domain config")
		dc.rc = cfg.Get()
		return nil
	})
	if err != nil {
		dc.r.log.WithError(err).Warning("failed to get cached config")
	}
	err = dc.fetchRemoteConfig()
	if err != nil {
		dc.r.log.WithError(err).Warning("failed to fetch config")
	}
	go func() {
		for range time.NewTicker(1 * time.Hour).C {
			err := dc.fetchRemoteConfig()
			if err != nil {
				dc.r.log.WithError(err).Warning("failed to update config")
			}
		}
	}()
}

func (dc *DomainConfig) fetchRemoteConfig() error {
	return State().ForBucket(dc.Domain).Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		api, err := dc.APIClient()
		if err != nil {
			return err
		}
		cfg, _, err := api.EndpointsApi.EndpointsAgentsConnectorsAgentConfigRetrieve(context.Background()).Execute()
		if err != nil {
			return err
		}
		dc.r.log.WithField("cap", cfg.SystemConfig.Capabilities).WithField("device_id", cfg.DeviceId).Debug("fetched remote config")
		dc.rc = cfg
		jc, err := cfg.MarshalJSON()
		if err != nil {
			return err
		}
		return b.Put([]byte("config"), jc)
	})
}

func (c *Config) loadDomains() error {
	c.log.Debug("Loading domains...")
	m, err := filepath.Glob(filepath.Join(c.DomainDir, "*.json"))
	if err != nil {
		c.log.WithError(err).Warning("failed to load domains")
		return err
	}
	dom := []*DomainConfig{}
	for _, match := range m {
		co, err := os.ReadFile(match)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		d := c.NewDomain()
		err = json.Unmarshal(co, &d)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		token, err := keyring.Get(keyring.Service("domain_token"), d.Domain, keyring.AccessibleAlways)
		if err != nil {
			if !errors.Is(err, keyring.ErrUnsupportedPlatform) {
				c.log.WithError(err).Warning("failed to load domain token from keyring")
			}
			token = d.FallbackToken
		}
		d.Token = token
		c.log.WithField("domain", d.Domain).Debug("loaded domain")
		dom = append(dom, d)
		d.loaded()
	}
	c.domains = dom
	c.log.Debug("Checking for managed domains...")
	return c.loadDomainsManaged()
}
