package config

import (
	"context"
	"fmt"
	"net/url"

	"goauthentik.io/api/v3"
)

type DomainConfig struct {
	Enabled            bool   `json:"enabled"`
	AuthentikURL       string `json:"authentik_url"`
	AppSlug            string `json:"app_slug"`
	Token              string `json:"token"`
	AuthenticationFlow string `json:"authentication_flow"`
	Domain             string `json:"domain"`

	c *api.APIClient
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
