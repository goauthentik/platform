package config

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
)

func TestDomain(config *api.AgentConfig, ac *api.APIClient) *DomainConfig {
	dc := &DomainConfig{
		rc: config,
	}
	if ac != nil {
		dc.c = ac
	}
	return dc
}

func TestDomainWithBrand(config *api.AgentConfig, ac *api.APIClient, brand *api.CurrentBrand) *DomainConfig {
	dc := TestDomain(config, ac)
	dc.brand = brand
	return dc
}

func TestAuthentikURL() string {
	if os.Getenv("CI") == "true" {
		return "http://localhost:9000"
	}
	return "http://authentik:9000"
}

func IntegrationDomain(t *testing.T) *DomainConfig {
	rc := &Config{}

	dc := &DomainConfig{
		Enabled:      true,
		AuthentikURL: TestAuthentikURL(),
		Domain:       "ak",
		Token:        "test-enroll-key",
		r:            rc.Default().(*Config),
	}

	assert.NoError(t, dc.Enroll(), "Failed to enroll integration domain")

	return dc
}
