package facts

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestGather(t *testing.T) {
	sysInfo, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)
	assert.NotNil(t, sysInfo)

	// Test JSON conversion
	jsonStr, err := sysInfo.MarshalJSON()
	assert.NoError(t, err)
	assert.NotEqual(t, string(jsonStr), "")
}

func TestSystemInfoStructure(t *testing.T) {
	sysInfo, err := Gather(common.TestingContext(t))
	assert.NoError(t, err)

	// Test that all major sections are present
	assert.NotEqual(t, sysInfo.Os.Get().Family, "")
	assert.NotEqual(t, sysInfo.Os.Get().Arch, "")
	assert.Greater(t, len(sysInfo.Disks), 0)
	assert.Greater(t, len(sysInfo.Network.Get().GetInterfaces()), 0)
}
