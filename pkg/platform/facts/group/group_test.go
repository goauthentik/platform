package group

import (
	"runtime"
	"strconv"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestGather(t *testing.T) {
	groups, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)

	assert.Greater(t, len(groups), 0)

	for _, group := range groups {
		assert.NotEqual(t, group.GetName(), "")
	}
}

func TestGatherDarwin(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("Skipping macOS-specific test")
	}

	groups, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// macOS specific tests
	expectedGroups := []string{"admin", "staff", "wheel"}
	foundExpected := make(map[string]bool)

	for _, group := range groups {
		assert.NotEqual(t, group.GetId(), "")
		assert.True(t, isNumeric(group.Id))

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

	groups, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Linux specific tests
	expectedGroups := []string{"root", "users", "sudo"}
	foundExpected := make(map[string]bool)
	foundRoot := false

	for _, group := range groups {
		if *group.Name == "root" {
			foundRoot = true
			assert.Equal(t, group.GetId(), "0")
		}

		assert.NotEqual(t, group.GetId(), "")
		assert.True(t, isNumeric(group.Id))

		// Check for common Linux groups
		for _, expected := range expectedGroups {
			if *group.Name == expected {
				foundExpected[expected] = true
			}
		}
	}

	assert.True(t, foundRoot)
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	groups, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Windows specific tests
	expectedGroups := []string{"Administrators", "Users"}
	foundExpected := make(map[string]bool)

	for _, group := range groups {
		assert.NotEqual(t, group.GetId(), "")
		assert.True(t, strings.HasPrefix(group.Id, "S-"))

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
