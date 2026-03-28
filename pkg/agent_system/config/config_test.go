package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/require"
	"goauthentik.io/platform/pkg/storage/state"
)

func TestSaveDomainPersistsFallbackToken(t *testing.T) {
	dir := t.TempDir()
	testState, err := state.Open(filepath.Join(dir, "state.db"), nil)
	require.NoError(t, err)
	st = testState
	t.Cleanup(func() {
		require.NoError(t, testState.Close())
		st = nil
	})

	cfg := &Config{
		DomainDir: dir,
		log:       (&Config{}).Default().(*Config).log,
	}

	dom := cfg.NewDomain()
	dom.Domain = "authentik"
	dom.AuthentikURL = "https://authentik.company"
	dom.Token = "device-token"

	require.NoError(t, cfg.SaveDomain(dom))

	raw, err := os.ReadFile(filepath.Join(dir, "authentik.json"))
	require.NoError(t, err)

	saved := &DomainConfig{}
	require.NoError(t, json.Unmarshal(raw, saved))
	require.Equal(t, "device-token", saved.FallbackToken)
}
