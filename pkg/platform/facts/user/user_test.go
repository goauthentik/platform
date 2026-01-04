package user

import (
	"runtime"
	"strconv"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestGather(t *testing.T) {
	users, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)

	assert.Greater(t, len(users), 0)

	for _, user := range users {
		assert.NotEqual(t, user.Id, "")
		assert.NotEqual(t, user.GetUsername(), "")
	}
}

func TestGatherDarwin(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("Skipping macOS-specific test")
	}

	users, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// macOS specific tests
	for _, user := range users {
		assert.NotEqual(t, user.GetId(), "")
		assert.True(t, isNumeric(user.Id))
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	users, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Linux specific tests
	foundRoot := false
	for _, user := range users {
		if user.Username != nil && *user.Username == "root" {
			foundRoot = true
			assert.Equal(t, user.GetId(), "0")
		}

		assert.NotEqual(t, user.GetId(), "")
		assert.True(t, isNumeric(user.Id))
	}

	assert.True(t, foundRoot)
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	users, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Windows specific tests
	for _, user := range users {
		assert.NotEqual(t, user.GetId(), "")
		assert.True(t, strings.HasPrefix(user.Id, "S-"))
	}
}

func isNumeric(s string) bool {
	_, err := strconv.Atoi(s)
	return err == nil
}
