package process

import (
	"runtime"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestGather(t *testing.T) {
	processes, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)

	if len(processes) == 0 {
		t.Skip("No processes found, skipping test")
	}

	for _, proc := range processes {
		assert.NotEqual(t, "", proc.Name, proc)
		assert.GreaterOrEqual(t, proc.Id, int32(0), "Process ID should be positive")
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	processes, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Linux specific tests
	foundKernel := false
	for _, proc := range processes {
		assert.NotEqual(t, "", proc.Name, proc.Name)
		if strings.Contains(proc.Name, "kthreadd") || strings.Contains(proc.Name, "systemd") {
			foundKernel = true
			break
		}
	}

	assert.True(t, foundKernel, "Expected to find kernel or system processes")
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	processes, err := gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Windows specific tests
	foundSystem := false
	for _, proc := range processes {
		if strings.Contains(strings.ToLower(proc.Name), "system") ||
			strings.Contains(strings.ToLower(proc.Name), "explorer") {
			foundSystem = true
			break
		}
	}

	assert.True(t, foundSystem, "Expected to find system processes")
}
