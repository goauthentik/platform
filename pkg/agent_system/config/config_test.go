package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/require"
	"goauthentik.io/platform/pkg/platform/keyring"
	"goauthentik.io/platform/pkg/storage/state"
)

func TestSaveDomainDoesNotPersistFallbackTokenWhenKeyringSucceeds(t *testing.T) {
	dir := t.TempDir()
	testState, err := state.Open(filepath.Join(dir, "state.db"), nil)
	require.NoError(t, err)
	st = testState
	oldSet := keyringSet
	oldGet := keyringGet
	t.Cleanup(func() {
		require.NoError(t, testState.Close())
		st = nil
		keyringSet = oldSet
		keyringGet = oldGet
	})

	keyringSet = func(service string, user string, access keyring.Accessibility, password string) error {
		return nil
	}
	keyringGet = func(service string, user string, access keyring.Accessibility) (string, error) {
		return "device-token", nil
	}

	cfg := &Config{
		DomainDir: dir,
		log:       (&Config{}).Default().(*Config).log,
	}

	dom := cfg.NewDomain()
	dom.Domain = "authentik"
	dom.AuthentikURL = "http://127.0.0.1:1"
	dom.Token = "device-token"

	require.NoError(t, cfg.SaveDomain(dom))

	raw, err := os.ReadFile(filepath.Join(dir, "authentik.json"))
	require.NoError(t, err)

	saved := &DomainConfig{}
	require.NoError(t, json.Unmarshal(raw, saved))
	require.Empty(t, saved.FallbackToken)
}

func TestSaveDomainPersistsFallbackTokenWhenKeyringUnsupported(t *testing.T) {
	dir := t.TempDir()
	testState, err := state.Open(filepath.Join(dir, "state.db"), nil)
	require.NoError(t, err)
	st = testState
	oldSet := keyringSet
	oldGet := keyringGet
	t.Cleanup(func() {
		require.NoError(t, testState.Close())
		st = nil
		keyringSet = oldSet
		keyringGet = oldGet
	})

	keyringSet = func(service string, user string, access keyring.Accessibility, password string) error {
		return keyring.ErrUnsupportedPlatform
	}
	keyringGet = func(service string, user string, access keyring.Accessibility) (string, error) {
		return "", keyring.ErrUnsupportedPlatform
	}

	cfg := &Config{
		DomainDir: dir,
		log:       (&Config{}).Default().(*Config).log,
	}

	dom := cfg.NewDomain()
	dom.Domain = "authentik"
	dom.AuthentikURL = "http://127.0.0.1:1"
	dom.Token = "device-token"

	require.NoError(t, cfg.SaveDomain(dom))

	raw, err := os.ReadFile(filepath.Join(dir, "authentik.json"))
	require.NoError(t, err)

	saved := &DomainConfig{}
	require.NoError(t, json.Unmarshal(raw, saved))
	require.Equal(t, "device-token", saved.FallbackToken)
}
