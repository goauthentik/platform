package group

import (
	"runtime"
	"strconv"
	"strings"
	"testing"
)

func TestGather(t *testing.T) {
	groups, err := Gather()
	if err != nil {
		t.Fatalf("Failed to gather group info: %v", err)
	}

	if len(groups) == 0 {
		t.Skip("No groups found, skipping test")
	}

	for _, group := range groups {
		if *group.Name == "" {
			t.Error("Group name should not be empty")
		}
	}

	t.Logf("Found %d groups", len(groups))
}

func TestGatherDarwin(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("Skipping macOS-specific test")
	}

	groups, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather group info on macOS: %v", err)
	}

	// macOS specific tests
	expectedGroups := []string{"admin", "staff", "wheel"}
	foundExpected := make(map[string]bool)

	for _, group := range groups {
		// macOS GIDs are typically numeric
		if group.Id != "" && !isNumeric(group.Id) {
			t.Errorf("Expected numeric GID on macOS, got: %s for %s", group.Id, *group.Name)
		}

		// Check for common macOS groups
		for _, expected := range expectedGroups {
			if *group.Name == expected {
				foundExpected[expected] = true
			}
		}
	}

	for _, expected := range expectedGroups {
		if !foundExpected[expected] {
			t.Logf("Expected to find group '%s' on macOS", expected)
		}
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	groups, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather group info on Linux: %v", err)
	}

	// Linux specific tests
	expectedGroups := []string{"root", "users", "sudo"}
	foundExpected := make(map[string]bool)
	foundRoot := false

	for _, group := range groups {
		if *group.Name == "root" {
			foundRoot = true
			if group.Id != "0" {
				t.Errorf("Expected root GID to be 0, got: %s", group.Id)
			}
		}

		// Linux GIDs should be numeric
		if group.Id != "" && !isNumeric(group.Id) {
			t.Errorf("Expected numeric GID on Linux, got: %s", group.Id)
		}

		// Check for common Linux groups
		for _, expected := range expectedGroups {
			if *group.Name == expected {
				foundExpected[expected] = true
			}
		}
	}

	if !foundRoot {
		t.Error("Expected to find root group on Linux")
	}
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	groups, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather group info on Windows: %v", err)
	}

	// Windows specific tests
	expectedGroups := []string{"Administrators", "Users"}
	foundExpected := make(map[string]bool)

	for _, group := range groups {
		// Windows SIDs should start with S- (if ID is provided)
		if group.Id != "" && !strings.HasPrefix(group.Id, "S-") {
			t.Logf("Windows group SID doesn't start with 'S-': %s", group.Id)
		}

		// Check for common Windows groups
		for _, expected := range expectedGroups {
			if *group.Name == expected {
				foundExpected[expected] = true
			}
		}
	}

	for _, expected := range expectedGroups {
		if !foundExpected[expected] {
			t.Logf("Expected to find group '%s' on Windows", expected)
		}
	}
}

func isNumeric(s string) bool {
	_, err := strconv.Atoi(s)
	return err == nil
}
