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

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  api.DEVICEFACTSOSFAMILY_LINUX,
		Name:    ptrStringIfNotBlank(name),
		Version: ptrStringIfNotBlank(version),
	}, nil
}

func getLinuxDistribution() (string, string) {
	// Try /etc/os-release first (systemd standard)
	if versionData, err := parseEnvFile("/etc/os-release"); err == nil {
		return extractVersion(versionData)
	}

	// Try /usr/lib/os-release as fallback
	if versionData, err := parseEnvFile("/usr/lib/os-release"); err == nil {
		return extractVersion(versionData)
	}

	// Try /etc/lsb-release (Ubuntu/Debian)
	if versionData, err := parseEnvFile("/etc/lsb-release"); err == nil {
		return extractVersion(versionData)
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

func parseEnvFile(filename string) (map[string]string, error) {
	vals := map[string]string{}
	file, err := os.Open(filename)
	if err != nil {
		return vals, err
	}
	defer func() {
		_ = file.Close()
	}()

	scanner := bufio.NewScanner(file)

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		key, value, ok := strings.Cut(line, "=")
		if !ok {
			continue
		}
		vals[key] = strings.Trim(value, "\"'")
	}
	return vals, nil
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
