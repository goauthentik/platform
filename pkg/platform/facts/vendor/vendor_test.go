package vendor

import (
	"runtime"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func TestVendor_Windows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip()
	}

	v := Gather(common.TestingContext(t))
	assert.NotEqual(t, v["rdp_cert_fingerprint"], "")
	assert.NotEqual(t, v["ssh_host_keys"], "")
}

func TestVendor_Linux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip()
	}

	v := Gather(common.TestingContext(t))
	assert.Equal(t, v["rdp_cert_fingerprint"], "")
	assert.NotEqual(t, v["ssh_host_keys"], "")
}
