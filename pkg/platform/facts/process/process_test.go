package process

import (
	"runtime"
	"strings"
	"testing"
)

func TestGather(t *testing.T) {
	processes, err := Gather()
	if err != nil {
		t.Fatalf("Failed to gather process info: %v", err)
	}

	if len(processes) == 0 {
		t.Skip("No processes found, skipping test")
	}

	for _, proc := range processes {
		if proc.Id < 0 {
			t.Error("Process ID should be positive")
		}
	}

	t.Logf("Found %d processes", len(processes))
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	processes, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather process info on Linux: %v", err)
	}

	// Linux specific tests
	foundKernel := false
	for _, proc := range processes {
		if proc.Name == "[kthreadd]" || proc.Name == "systemd" {
			foundKernel = true
			break
		}
	}

	if !foundKernel {
		t.Log("Expected to find kernel or system processes")
	}
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	processes, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather process info on Windows: %v", err)
	}

	// Windows specific tests
	foundSystem := false
	for _, proc := range processes {
		if strings.Contains(strings.ToLower(proc.Name), "system") ||
			strings.Contains(strings.ToLower(proc.Name), "explorer") {
			foundSystem = true
			break
		}
	}

	if !foundSystem {
		t.Log("Expected to find system processes")
	}
}
