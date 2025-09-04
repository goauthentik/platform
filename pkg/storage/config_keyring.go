package storage

import (
	"fmt"

	"goauthentik.io/cli/pkg/storage/keyring"
)

func keyringSvc(name string) string {
	return fmt.Sprintf("io.goauthentik.agent.%s", name)
}

func (cfg *ConfigManager) loadKeyring() error {
	for name, profile := range cfg.loaded.Profiles {
		l := cfg.log.WithField("profile", name)
		l.Debug("Getting access token from keyring")
		v, err := keyring.Get(keyringSvc("access_token"), name)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		profile.AccessToken = v
		l.Debug("Getting refresh token from keyring")
		v, err = keyring.Get(keyringSvc("refresh_token"), name)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		profile.RefreshToken = v
	}
	return nil
}

func (cfg *ConfigManager) saveKeyring() error {
	for name, profile := range cfg.loaded.Profiles {
		l := cfg.log.WithField("profile", name)
		l.Debug("Setting access token in keyring")
		err := keyring.Set(keyringSvc("access_token"), name, profile.AccessToken)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		l.Debug("Setting refresh token in keyring")
		err = keyring.Set(keyringSvc("refresh_token"), name, profile.RefreshToken)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
	}
	return nil

}
