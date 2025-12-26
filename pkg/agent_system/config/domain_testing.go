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

func IntegrationDomain(t *testing.T) *DomainConfig {
	ak := "http://authentik:9000"
	if os.Getenv("CI") == "true" {
		ak = "http://localhost:9000"
	}
	rc := &Config{}

	dc := &DomainConfig{
		Enabled:      true,
		AuthentikURL: ak,
		Domain:       "ak",
		Token:        "test-enroll-key",
		r:            rc.Default().(*Config),
	}

	assert.NoError(t, dc.Enroll(), "Failed to enroll integration domain")

	return dc
}
