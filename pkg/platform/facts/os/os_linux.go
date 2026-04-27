//go:build linux

package os

import (
	"bufio"
	"os"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	name, version := getLinuxDistribution()

	info := api.DeviceFactsRequestOs{
		Arch:   runtime.GOARCH,
		Family: api.DEVICEFACTSOSFAMILY_LINUX,
		Name:   api.PtrString(name),
	}
	if version != "" {
		info.Version = api.PtrString(version)
	}

	return info, nil
}

func getLinuxDistribution() (string, string) {
	// Try /etc/os-release first (systemd standard)
	if name, version := parseOSRelease("/etc/os-release"); name != "" {
		return name, version
	}

	// Try /usr/lib/os-release as fallback
	if name, version := parseOSRelease("/usr/lib/os-release"); name != "" {
		return name, version
	}

	// Try /etc/lsb-release (Ubuntu/Debian)
	if name, version := parseLSBRelease(); name != "" {
		return name, version
	}

	// Try various distribution-specific files
	distFiles := map[string]string{
		"/etc/redhat-release": "Red Hat",
		"/etc/centos-release": "CentOS",
		"/etc/fedora-release": "Fedora",
		"/etc/debian_version": "Debian",
		"/etc/arch-release":   "Arch Linux",
	}

	for file, distName := range distFiles {
		if content := readFirstLine(file); content != "" {
			return distName, content
		}
	}

	return "Linux", getKernelVersion()
}

func parseOSRelease(filename string) (string, string) {
	file, err := os.Open(filename)
	if err != nil {
		return "", ""
	}
	defer func() {
		_ = file.Close()
	}()

	scanner := bufio.NewScanner(file)
	var name, prettyName, versionID, version, buildID string

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		key, value, ok := strings.Cut(line, "=")
		if !ok {
			continue
		}
		value = strings.Trim(value, "\"'")
		switch key {
		case "NAME":
			name = value
		case "PRETTY_NAME":
			prettyName = value
		case "VERSION_ID":
			versionID = value
		case "VERSION":
			version = value
		case "BUILD_ID":
			buildID = value
		}
	}

	fullName := prettyName
	if fullName == "" {
		fullName = name
	}
	if fullName == "" {
		return "", ""
	}

	parsedName, parsedVersion := extractVersion(fullName)
	if parsedVersion != "" {
		return parsedName, parsedVersion
	}
	if versionID != "" {
		return parsedName, versionID
	}
	if version != "" {
		return parsedName, version
	}
	if buildID != "" {
		return parsedName, buildID
	}

	return parsedName, ""
}

func parseLSBRelease() (string, string) {
	file, err := os.Open("/etc/lsb-release")
	if err != nil {
		return "", ""
	}
	defer func() {
		_ = file.Close()
	}()

	var name, version string
	scanner := bufio.NewScanner(file)

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "DISTRIB_ID=") {
			name = strings.TrimPrefix(line, "DISTRIB_ID=")
		} else if strings.HasPrefix(line, "DISTRIB_RELEASE=") {
			version = strings.TrimPrefix(line, "DISTRIB_RELEASE=")
		}
	}

	return name, version
}

func readFirstLine(filename string) string {
	file, err := os.Open(filename)
	if err != nil {
		return ""
	}
	defer func() {
		_ = file.Close()
	}()

	scanner := bufio.NewScanner(file)
	if scanner.Scan() {
		return strings.TrimSpace(scanner.Text())
	}

	return ""
}

func getKernelVersion() string {
	content := readFirstLine("/proc/version")
	if content != "" {
		parts := strings.Fields(content)
		if len(parts) >= 3 {
			return parts[2]
		}
	}

	return ""
}
