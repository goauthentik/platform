package user

import (
	"runtime"
	"strconv"
	"strings"
	"testing"

	"goauthentik.io/api/v3"
)

func TestGather(t *testing.T) {
	users, err := Gather()
	if err != nil {
		t.Fatalf("Failed to gather user info: %v", err)
	}

	if len(users) == 0 {
		t.Skip("No users found, skipping test")
	}

	for _, user := range users {
		if user.Id == "" {
			t.Error("User ID should not be empty")
		}

		if user.Username == api.PtrString("") {
			t.Error("Username should not be empty")
		}
	}

	t.Logf("Found %d users", len(users))
}

func TestGatherDarwin(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("Skipping macOS-specific test")
	}

	users, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather user info on macOS: %v", err)
	}

	// macOS specific tests
	for _, user := range users {
		// macOS UIDs are typically numeric
		if user.Id != "" && !isNumeric(user.Id) {
			t.Errorf("Expected numeric UID on macOS, got: %s", user.Id)
		}
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	users, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather user info on Linux: %v", err)
	}

	// Linux specific tests
	foundRoot := false
	for _, user := range users {
		if user.Username != nil && *user.Username == "root" {
			foundRoot = true
			if user.Id != "0" {
				t.Errorf("Expected root UID to be 0, got: %s", user.Id)
			}
		}

		// Linux UIDs should be numeric
		if user.Id != "" && !isNumeric(user.Id) {
			t.Errorf("Expected numeric UID on Linux, got: %s", user.Id)
		}
	}

	if !foundRoot {
		t.Error("Expected to find root user on Linux")
	}
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	users, err := gather()
	if err != nil {
		t.Fatalf("Failed to gather user info on Windows: %v", err)
	}

	// Windows specific tests
	for _, user := range users {
		// Windows SIDs should start with S-
		if user.Id != "" && !strings.HasPrefix(user.Id, "S-") {
			t.Errorf("Expected Windows SID to start with 'S-', got: %s", user.Id)
		}
	}
}

func isNumeric(s string) bool {
	_, err := strconv.Atoi(s)
	return err == nil
}
