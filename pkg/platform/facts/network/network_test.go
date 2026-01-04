package network

import (
	"runtime"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGather(t *testing.T) {
	info, err := Gather()
	assert.NoError(t, err)

	assert.NotEqual(t, info.Hostname, "")
	assert.Greater(t, len(info.GetInterfaces()), 0)

	for _, iface := range info.Interfaces {
		assert.NotEqual(t, iface.Name, "")
		assert.Greater(t, len(iface.IpAddresses), 0)
	}
}

func TestGatherLinux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("Skipping Linux-specific test")
	}

	info, err := gather()
	assert.NoError(t, err)
	assert.NotNil(t, info)
}

func TestGatherWindows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("Skipping Windows-specific test")
	}

	info, err := gather()
	assert.NoError(t, err)
	assert.NotNil(t, info)
}
