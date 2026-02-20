//go:build windows

package managedconfig

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/sys/windows/registry"
)

func TestWindows(t *testing.T) {
	rk := `SOFTWARE\authentik Security Inc.\Platform-testing`
	// setup
	_, _, err := registry.CreateKey(registry.LOCAL_MACHINE, rk, 0xF003F)
	assert.NoError(t, err)

	k, err := registry.OpenKey(registry.LOCAL_MACHINE, rk, registry.WRITE)
	assert.NoError(t, err)
	assert.NoError(t, k.SetStringValue("foo", "some value"))
	t.Cleanup(func() {
		registry.DeleteKey(registry.LOCAL_MACHINE, rk)
	})

	type Config struct {
		Foo string `registry:"foo"`
		Bar string `registry:"baz"`
	}

	p, err := Get[Config](pstr.PlatformString{
		Windows: &rk,
	})
	assert.NoError(t, err)
	assert.Equal(t, "some value", p.Foo)
	assert.Equal(t, "", p.Bar)
}
