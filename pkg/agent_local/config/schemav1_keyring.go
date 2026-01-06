package config

import (
	"errors"

	"goauthentik.io/platform/pkg/platform/keyring"
)

func getKeyringDefault(svc string, name string, def string) (string, error) {
	v, err := keyring.Get(keyring.Service(svc), name, keyring.AccessibleUser)
	if err != nil {
		if errors.Is(err, keyring.ErrUnsupportedPlatform) {
			return def, nil
		}
		return "", err
	}
	return v, nil
}

func (c ConfigV1) PostLoad() error {
	for name, profile := range c.Profiles {
		l := c.log.WithField("profile", name)
		l.Debug("Getting access token from keyring")
		v, err := getKeyringDefault("access_token", name, profile.FallbackAccessToken)
		if err != nil {
			l.WithError(err).Warning("failed to get keyring")
			return err
		}
		profile.AccessToken = v
		l.Debug("Getting refresh token from keyring")
		v, err = getKeyringDefault("refresh_token", name, profile.FallbackRefreshToken)
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
		err := keyring.Set(keyring.Service("access_token"), name, keyring.AccessibleUser, profile.AccessToken)
		if err != nil {
			if errors.Is(err, keyring.ErrUnsupportedPlatform) {
				profile.FallbackAccessToken = profile.AccessToken
			} else {
				l.WithError(err).Warning("failed to set keyring")
				return err
			}
		}
		l.Debug("Setting refresh token in keyring")
		err = keyring.Set(keyring.Service("refresh_token"), name, keyring.AccessibleUser, profile.RefreshToken)
		if err != nil {
			if errors.Is(err, keyring.ErrUnsupportedPlatform) {
				profile.FallbackRefreshToken = profile.RefreshToken
			} else {
				l.WithError(err).Warning("failed to set keyring")
				return err
			}
		}
	}
	return nil
}
