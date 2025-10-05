package storage

import (
	"goauthentik.io/cli/pkg/storage/keyring"
)

func (c ConfigV1) PostLoad() error {
	for name, profile := range c.Profiles {
		l := c.log.WithField("profile", name)
		l.Debug("Getting access token from keyring")
		v, err := keyring.Get(keyring.Service("access_token"), name)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		profile.AccessToken = v
		l.Debug("Getting refresh token from keyring")
		v, err = keyring.Get(keyring.Service("refresh_token"), name)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		profile.RefreshToken = v
	}
	return nil
}

func (c ConfigV1) PreSave() error {
	for name, profile := range c.Profiles {
		l := c.log.WithField("profile", name)
		l.Debug("Setting access token in keyring")
		err := keyring.Set(keyring.Service("access_token"), name, profile.AccessToken)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		l.Debug("Setting refresh token in keyring")
		err = keyring.Set(keyring.Service("refresh_token"), name, profile.RefreshToken)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
	}
	return nil
}
