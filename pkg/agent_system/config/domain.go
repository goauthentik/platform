package config

import (
	"context"
	"encoding/json"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"time"

	"go.etcd.io/bbolt"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/keyring"
)

type DomainConfig struct {
	Enabled            bool   `json:"enabled"`
	AuthentikURL       string `json:"authentik_url"`
	AppSlug            string `json:"app_slug"`
	AuthenticationFlow string `json:"authentication_flow"`
	Domain             string `json:"domain"`

	// Saved to keyring
	Token string `json:"-"`

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
	apiConfig.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", dc.Token))

	c := api.NewAPIClient(apiConfig)
	dc.c = c
	return dc.c, nil
}

func (dc DomainConfig) Test() error {
	ac, err := dc.APIClient()
	if err != nil {
		return err
	}
	_, _, err = ac.EndpointsApi.EndpointsAgentsConnectorsAgentConfigRetrieve(context.Background()).Execute()
	if err != nil {
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
	State().ForBucket(dc.Domain).View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		cfg := api.NullableAgentConfig{}
		err := cfg.UnmarshalJSON(b.Get([]byte("config")))
		if err != nil {
			return err
		}
		dc.r.log.Info("Loaded domain config")
		dc.rc = cfg.Get()
		return nil
	})
	dc.fetchRemoteConfig()
	go func() {
		for range time.NewTicker(1 * time.Hour).C {
			dc.fetchRemoteConfig()
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
		dc.r.log.Debug("fetched remote config")
		dc.rc = cfg
		jc, err := cfg.MarshalJSON()
		if err != nil {
			return err
		}
		b.Put([]byte("config"), jc)
		return nil
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
		token, err := keyring.Get(keyring.Service("domain_token"), d.Domain)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain token from keyring")
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
