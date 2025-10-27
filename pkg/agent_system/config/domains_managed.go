package config

import (
	"strings"

	"github.com/pkg/errors"
	managedconfig "goauthentik.io/platform/pkg/platform/managed_config"
	"goauthentik.io/platform/pkg/platform/pstr"
)

type SysdManagedConfig struct {
	RegistrationToken string `registry:"registration_token"`
	URL               string `registry:"url"`
}

const managedDomainName = "ak-mdm-managed"

func (c *Config) loadDomainsManaged() error {
	mc, err := managedconfig.Get[SysdManagedConfig](pstr.PlatformString{
		Darwin:  pstr.S("io.goauthentik.platform"),
		Windows: pstr.S(`SOFTWARE\authentik Security Inc.\Platform`),
	})
	if err != nil {
		if errors.Is(err, managedconfig.ErrNotFound) || errors.Is(err, managedconfig.ErrNotSupported) {
			return nil
		}
		return errors.Wrap(err, "failed to load managed config")
	}
	// Check if we already have a managed domain
	for _, d := range c.domains {
		if d.Domain != managedDomainName {
			continue
		}
		if strings.EqualFold(d.AuthentikURL, mc.URL) {
			// Found a managed domain with the same authentik URL, do nothing
			c.log.Debug("resumed existing managed domain")
			return nil
		}
		// Found managed domain, different authentik URL
		err = d.Delete()
		if err != nil {
			c.log.WithError(err).Warning("failed to delete old managed domain")
			continue
		}
	}
	d := DomainConfig{
		Enabled:            true,
		AuthentikURL:       mc.URL,
		AppSlug:            "",
		Token:              mc.RegistrationToken,
		AuthenticationFlow: "default-authentication-flow",
		Domain:             managedDomainName,
	}
	if err := d.Test(); err != nil {
		return errors.Wrap(err, "failed to test domain")
	}
	return c.SaveDomain(d)
}
