package os

import (
	stdos "os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
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
	if info.Version != nil {
		assert.NotEqual(t, strings.TrimSpace(*info.Version), "")
	}
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

func TestParseOSRelease(t *testing.T) {
	for _, tc := range []struct {
		name    string
		content string
		osName  string
		version string
	}{
		{
			name: "pretty name with embedded version",
			content: `NAME="Ubuntu"
PRETTY_NAME="Ubuntu 24.04.3 LTS"
`,
			osName:  "Ubuntu",
			version: "24.04.3 LTS",
		},
		{
			name: "fallback to version id",
			content: `NAME="TestOS"
PRETTY_NAME="TestOS"
VERSION_ID="1.2"
VERSION="rolling"
BUILD_ID="2025.03.19"
`,
			osName:  "TestOS",
			version: "1.2",
		},
		{
			name: "fallback to build id",
			content: `NAME="EndeavourOS"
PRETTY_NAME="EndeavourOS"
BUILD_ID="2025.03.19"
`,
			osName:  "EndeavourOS",
			version: "2025.03.19",
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			path := filepath.Join(t.TempDir(), "os-release")
			err := stdos.WriteFile(path, []byte(tc.content), 0o600)
			assert.NoError(t, err)

			name, version := parseOSRelease(path)
			assert.Equal(t, tc.osName, name)
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
