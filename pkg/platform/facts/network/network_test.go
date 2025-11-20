package network

import (
	"runtime"
	"strings"
	"testing"
)

func TestGather(t *testing.T) {
	info, err := Gather()
	if err != nil {
		t.Fatalf("Failed to gather network info: %v", err)
	}

	if info.Hostname == "" {
		t.Error("Hostname is empty")
	}

	if len(info.Interfaces) == 0 {
		t.Skip("No network interfaces found, skipping test")
	}

	for _, iface := range info.Interfaces {
		if iface.Name == "" {
			t.Error("Interface name is empty")
		}

		if len(iface.IpAddresses) == 0 {
			t.Error("Interface IP address is empty")
		}
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	info, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather network info on Linux: %v", err)
	}

	// Linux specific tests
	for _, iface := range info.Interfaces {
		if strings.HasPrefix(iface.Name, "eth") ||
			strings.HasPrefix(iface.Name, "wlan") ||
			strings.HasPrefix(iface.Name, "enp") {
			// Valid Linux interface naming
		} else {
			t.Logf("Unexpected interface name format: %s", iface.Name)
		}
	}
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	info, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather network info on Windows: %v", err)
	}

	// Windows specific tests
	t.Logf("Found %d network interfaces", len(info.Interfaces))
	t.Logf("Firewall enabled: %v", info.FirewallEnabled)
}
