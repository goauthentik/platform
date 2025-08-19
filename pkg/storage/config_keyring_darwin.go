//go:build darwin
// +build darwin

package storage

import (
	"github.com/keybase/go-keychain"
)

const (
	keyringService = "io.goauthentik.agent"
)

func (cfg *ConfigManager) loadKeyring() error {
	for _, profile := range cfg.loaded.Profiles {
		acc, err := keychain.GetGenericPassword(keyringService, profile.keyringAccessTokenName(), "", "")
		if err != nil {
			return err
		}
		profile.AccessToken = string(acc)
		ref, err := keychain.GetGenericPassword(keyringService, profile.keyringRefreshTokenName(), "", "")
		if err != nil {
			return err
		}
		profile.RefreshToken = string(ref)
	}
	return nil
}

func (cfg *ConfigManager) saveKeyring() error {
	for _, profile := range cfg.loaded.Profiles {
		item := keychain.NewGenericPassword(keyringService, profile.keyringAccessTokenName(), "", []byte(profile.AccessToken), "")
		item.SetSynchronizable(keychain.SynchronizableNo)
		item.SetAccessible(keychain.AccessibleWhenUnlocked)
		err := keychain.AddItem(item)
		if err != nil {
			return err
		}
		item = keychain.NewGenericPassword(keyringService, profile.keyringRefreshTokenName(), "", []byte(profile.RefreshToken), "")
		item.SetSynchronizable(keychain.SynchronizableNo)
		item.SetAccessible(keychain.AccessibleWhenUnlocked)
		err = keychain.AddItem(item)
		if err != nil {
			return err
		}
	}
	return nil
}
