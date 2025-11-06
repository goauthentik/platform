package state_test

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/storage/state"
)

func TestKey(t *testing.T) {
	k := (&state.Key{}).Add("foo", "bar")
	assert.Equal(t, "/foo/bar", k.String())
	k = (&state.Key{}).Add("foo", "bar").Prefix(true)
	assert.Equal(t, "/foo/bar/", k.String())
	k = (&state.Key{}).Add("foo", "bar").Prefix(true).Up()
	assert.Equal(t, "/foo/", k.String())
}

func TestKeyCopy(t *testing.T) {
	k := (&state.Key{}).Add("foo", "bar")
	assert.Equal(t, "/foo/bar", k.String())
	assert.Equal(t, "/foo/bar", k.Copy().String())
}

func TestKeyParse(t *testing.T) {
	assert.Equal(t, "/foo/bar", state.KeyFromString("/foo/bar").String())
	assert.True(t, state.KeyFromString("/foo/bar/").IsPrefix())
}
