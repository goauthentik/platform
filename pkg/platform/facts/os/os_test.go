package os

import (
	"encoding/json"
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
		id      string
		raw     map[string]string
		name    string
		version string
	}{
		{
			id: "ubuntu-24-04-lsb",
			// docker run -it --rm ubuntu:24.04 cat /etc/lsb-release
			raw: map[string]string{
				"DISTRIB_ID":          "Ubuntu",
				"DISTRIB_RELEASE":     "24.04",
				"DISTRIB_CODENAME":    "noble",
				"DISTRIB_DESCRIPTION": "Ubuntu 24.04.4 LTS",
			},
			name:    "Ubuntu",
			version: "24.04",
		},
		{
			id: "ubuntu-24-04-os",
			// docker run -it --rm ubuntu:24.04 cat /etc/os-release
			raw: map[string]string{
				"PRETTY_NAME":        "Ubuntu 24.04.4 LTS",
				"NAME":               "Ubuntu",
				"VERSION_ID":         "24.04",
				"VERSION":            "24.04.4 LTS (Noble Numbat)",
				"VERSION_CODENAME":   "noble",
				"ID":                 "ubuntu",
				"ID_LIKE":            "debian",
				"HOME_URL":           "https://www.ubuntu.com/",
				"SUPPORT_URL":        "https://help.ubuntu.com/",
				"BUG_REPORT_URL":     "https://bugs.launchpad.net/ubuntu/",
				"PRIVACY_POLICY_URL": "https://www.ubuntu.com/legal/terms-and-policies/privacy-policy",
				"UBUNTU_CODENAME":    "noble",
				"LOGO":               "ubuntu-logo",
			},
			name:    "Ubuntu",
			version: "24.04",
		},
		{
			id: "fedora-44-os",
			// docker run -it --rm fedora cat /etc/os-release
			raw: map[string]string{
				"NAME":                            "Fedora Linux",
				"VERSION":                         "44 (Container Image)",
				"RELEASE_TYPE":                    "stable",
				"ID":                              "fedora",
				"VERSION_ID":                      "44",
				"VERSION_CODENAME":                "",
				"PRETTY_NAME":                     "Fedora Linux 44 (Container Image)",
				"ANSI_COLOR":                      "0;38;2;60;110;180",
				"LOGO":                            "fedora-logo-icon",
				"CPE_NAME":                        "cpe:/o:fedoraproject:fedora:44",
				"DEFAULT_HOSTNAME":                "fedora",
				"HOME_URL":                        "https://fedoraproject.org/",
				"DOCUMENTATION_URL":               "https://docs.fedoraproject.org/en-US/fedora/f44/",
				"SUPPORT_URL":                     "https://ask.fedoraproject.org/",
				"BUG_REPORT_URL":                  "https://bugzilla.redhat.com/",
				"REDHAT_BUGZILLA_PRODUCT":         "Fedora",
				"REDHAT_BUGZILLA_PRODUCT_VERSION": "44",
				"REDHAT_SUPPORT_PRODUCT":          "Fedora",
				"REDHAT_SUPPORT_PRODUCT_VERSION":  "44",
				"SUPPORT_END":                     "2027-05-19",
				"VARIANT":                         "Container Image",
				"VARIANT_ID":                      "container",
			},
			name:    "Fedora Linux",
			version: "44",
		},
		{
			id: "debian-sid-os",
			// docker run -it --rm debian:sid cat /etc/os-release
			raw: map[string]string{
				"PRETTY_NAME":      "Debian GNU/Linux forky/sid",
				"NAME":             "Debian GNU/Linux",
				"VERSION_CODENAME": "forky",
				"ID":               "debian",
				"HOME_URL":         "https://www.debian.org/",
				"SUPPORT_URL":      "https://www.debian.org/support",
				"BUG_REPORT_URL":   "https://bugs.debian.org/",
			},
			name:    "Debian GNU/Linux",
			version: "forky",
		},
	} {
		t.Run(tc.id, func(t *testing.T) {
			name, version := extractVersion(tc.raw)
			assert.Equal(t, tc.name, name)
			assert.Equal(t, tc.version, version)
		})
	}
}

func TestPtrStringIfNotBlank(t *testing.T) {
	assert.Nil(t, ptrStringIfNotBlank(""))
	assert.Nil(t, ptrStringIfNotBlank(" \t\n"))

	value := ptrStringIfNotBlank("  24.04  ")
	assert.NotNil(t, value)
	assert.Equal(t, "24.04", *value)
}

func TestMarshalOmitsBlankOptionalStrings(t *testing.T) {
	data, err := json.Marshal(api.DeviceFactsRequestOs{
		Arch:    "amd64",
		Family:  api.DEVICEFACTSOSFAMILY_LINUX,
		Name:    ptrStringIfNotBlank("Linux"),
		Version: ptrStringIfNotBlank(""),
	})
	assert.NoError(t, err)
	assert.JSONEq(t, `{"arch":"amd64","family":"linux","name":"Linux"}`, string(data))
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
