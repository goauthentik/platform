package config

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak"
)

func TestDomain(config *api.AgentConfig, ac *ak.TestAPIClient) *DomainConfig {
	dc := &DomainConfig{
		rc: config,
	}
	if ac != nil {
		dc.c = ac.APIClient
	}
	return dc
}
