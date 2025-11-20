package os

import (
	"runtime"
	"testing"
)

func TestGather(t *testing.T) {
	info, err := Gather()
	if err != nil {
		t.Fatalf("Failed to gather OS info: %v", err)
	}

	if info.Arch == "" {
		t.Error("Architecture is empty")
	}

	if info.Family == "" {
		t.Error("OS family is empty")
	}

	expectedFamily := runtime.GOOS
	if string(info.Family) != expectedFamily {
		t.Errorf("Expected family %s, got %s", expectedFamily, info.Family)
	}

	if info.Arch != runtime.GOARCH {
		t.Errorf("Expected arch %s, got %s", runtime.GOARCH, info.Arch)
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	info, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather OS info on Linux: %v", err)
	}

	if info.Family != "linux" {
		t.Errorf("Expected family 'linux', got '%s'", info.Family)
	}

	if *info.Name == "" {
		t.Error("OS name should not be empty on Linux")
	}
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	info, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather OS info on Windows: %v", err)
	}

	if info.Family != "windows" {
		t.Errorf("Expected family 'windows', got '%s'", info.Family)
	}

	if *info.Name == "" {
		t.Error("OS name should not be empty on Windows")
	}
}
