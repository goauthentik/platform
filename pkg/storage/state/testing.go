package state

import (
	"os"
	"path"
	"testing"

	"github.com/stretchr/testify/assert"
)

var testState *State

func TestState(t *testing.T) *State {
	t.Helper()
	if testState == nil {
		statePath := path.Join(t.TempDir(), "ak-state.db")
		sst, err := Open(statePath, nil)
		assert.NoError(t, err)
		testState = sst
		t.Cleanup(func() {
			assert.NoError(t, testState.Close())
			assert.NoError(t, os.Remove(statePath))
			testState = nil
		})
	}
	return testState
}
