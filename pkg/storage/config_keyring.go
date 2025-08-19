//go:build !darwin
// +build !darwin

package storage

import (
	"github.com/zalando/go-keyring"
)

const (
	keyringService = "io.goauthentik.agent"
)

func (cfg *ConfigManager) loadKeyring() error {
	for _, profile := range cfg.loaded.Profiles {
		v, err := keyring.Get(keyringService, profile.keyringAccessTokenName())
		if err != nil {
			return err
		}
		profile.AccessToken = v
		v, err = keyring.Get(keyringService, profile.keyringRefreshTokenName())
		if err != nil {
			return err
		}
		profile.RefreshToken = v
	}
	return nil
}

func (cfg *ConfigManager) saveKeyring() error {
	service := "io.goauthentik.agent"
	for _, profile := range cfg.loaded.Profiles {
		err := keyring.Set(service, profile.keyringAccessTokenName(), profile.AccessToken)
		if err != nil {
			return err
		}
		err = keyring.Set(service, profile.keyringRefreshTokenName(), profile.RefreshToken)
		if err != nil {
			return err
		}
	}
	return nil

}
