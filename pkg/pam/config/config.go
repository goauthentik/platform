package config

import (
	"fmt"
	"net/url"
	"os"

	"goauthentik.io/api/v3"
	"gopkg.in/yaml.v3"
)

type Config struct {
	AuthentikURL string `yaml:"authentik_url"`
	Insecure     bool   `yaml:"insecure"`
	FlowSlug     string `yaml:"authentication_flow"`
	Debug        bool   `yaml:"debug"`
	TokenJWKS    string `yaml:"token_jwks"`

	client *api.APIClient
}

func (c *Config) API() *api.APIClient {
	return c.client
}

var c *Config

func Get() *Config {
	return c
}

func Load() error {
	rawConfig, err := os.ReadFile("/etc/authentik/pam.yaml")
	if err != nil {
		return err
	}
	err = yaml.Unmarshal([]byte(rawConfig), &c)
	if err != nil {
		return err
	}

	akURL, err := url.Parse(c.AuthentikURL)
	if err != nil {
		return err
	}

	config := api.NewConfiguration()
	config.Debug = true
	config.UserAgent = fmt.Sprintf("goauthentik.io/cli@%s", "test")
	config.Host = akURL.Host
	config.Scheme = akURL.Scheme
	// config.HTTPClient = &http.Client{
	// 	Transport: GetTLSTransport(c.Insecure),
	// }

	// config.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", c.Token))
	apiClient := api.NewAPIClient(config)
	c.client = apiClient
	return nil
}
