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
	assert.Regexp(t, `(\d+\.(?:\d+\.?)+)`, *info.Version, "Version must only contain numbers: '%s'", *info.Version)
}

func TestExtract(t *testing.T) {
	for _, tc := range []struct {
		raw     string
		name    string
		version string
	}{
		{
			raw:     "Ubuntu 24.04.3 LTS",
			name:    "Ubuntu",
			version: "24.04.3 LTS",
		},
		{
			raw:     "Fedora Linux 43 (Workstation Edition)",
			name:    "Fedora Linux",
			version: "43 (Workstation Edition)",
		},
	} {
		t.Run(tc.raw, func(t *testing.T) {
			name, version := extractVersion(tc.raw)
			assert.Equal(t, tc.name, name)
			assert.Equal(t, tc.version, version)
		})
	}
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
