package config

import (
	"strings"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/cli/setup"
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
		Windows: pstr.S(`SOFTWARE\authentik Security Inc.\Platform\ManagedConfig`),
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
	// Enroll in managed config domain
	d := c.NewDomain()
	d.AuthentikURL = mc.URL
	d.Token = mc.RegistrationToken
	d.AppSlug = setup.DefaultAppSlug
	d.AuthenticationFlow = "default-authentication-flow"
	d.Domain = managedDomainName
	d.Managed = true
	err = d.Enroll()
	if err != nil {
		return errors.Wrap(err, "failed to enroll")
	}
	err = c.SaveDomain(d)
	if err != nil {
		return errors.Wrap(err, "failed to save domain")
	}
	d.loaded()
	c.domains = append(c.domains, d)
	return nil
}
