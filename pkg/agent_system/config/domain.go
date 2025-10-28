package config

import (
	"context"
	"encoding/json"
	"fmt"
	"net/url"
	"os"
	"path/filepath"

	"github.com/pkg/errors"
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

	c *api.APIClient
	r *Config
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
	_, _, err = ac.CoreApi.CoreUsersMeRetrieve(context.Background()).Execute()
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

func (c *Config) NewDomain() DomainConfig {
	return DomainConfig{
		Enabled: true,
		r:       c,
	}
}

func (c *Config) loadDomains() error {
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
		d := c.NewDomain()
		err = json.Unmarshal(co, &d)
		if err != nil {
			c.log.WithError(err).Warning("failed to load domain")
			continue
		}
		token, err := keyring.Get(keyring.Service("domain_token"), d.Domain)
		if err != nil {
			if !errors.Is(err, keyring.ErrUnsupportedPlatform) {
				c.log.WithError(err).Warning("failed to load domain token")
				continue
			}
			token = d.FallbackToken
		}
		d.Token = token
		c.log.WithField("domain", d.Domain).Debug("loaded domain")
		dom = append(dom, d)
	}
	c.domains = dom
	c.log.Debug("Checking for managed domains...")
	return c.loadDomainsManaged()
}
