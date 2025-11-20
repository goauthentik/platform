//go:build linux

package os

import (
	"bufio"
	"os"
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestOs, error) {
	name, version := getLinuxDistribution()

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  api.DEVICEFACTSOSFAMILY_LINUX,
		Name:    api.PtrString(name),
		Version: api.PtrString(version),
	}, nil
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
	defer file.Close()

	var name, version string
	scanner := bufio.NewScanner(file)

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "NAME=") {
			name = strings.Trim(strings.TrimPrefix(line, "NAME="), "\"")
		} else if strings.HasPrefix(line, "VERSION=") {
			version = strings.Trim(strings.TrimPrefix(line, "VERSION="), "\"")
		}
	}

	return name, version
}

func parseLSBRelease() (string, string) {
	file, err := os.Open("/etc/lsb-release")
	if err != nil {
		return "", ""
	}
	defer file.Close()

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
	defer file.Close()

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
