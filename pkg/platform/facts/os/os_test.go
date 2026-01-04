package os

import (
	"runtime"
	"slices"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestGather(t *testing.T) {
	info, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)

	assert.NotEqual(t, info.Arch, "")
	assert.NotEqual(t, info.Family, "")
	assert.True(t, slices.Contains(api.AllowedDeviceFactsOSFamilyEnumValues, info.Family))
	assert.Equal(t, info.Arch, runtime.GOARCH)
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	info, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	assert.Equal(t, info.Family, api.DEVICEFACTSOSFAMILY_LINUX)
	assert.NotEqual(t, info.GetName(), "")
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	info, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	assert.Equal(t, info.Family, api.DEVICEFACTSOSFAMILY_WINDOWS)
	assert.NotEqual(t, info.GetName(), "")
}
