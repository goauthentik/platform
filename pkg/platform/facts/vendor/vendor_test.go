package vendor

import (
	"runtime"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestVendor_Windows(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip()
	}

	v := Gather()
	assert.NotEqual(t, "", v["rdp_cert_fingerprint"])
	assert.NotEqual(t, "", v["ssh_host_keys"])
}

func TestVendor_Linux(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip()
	}

	v := Gather()
	assert.Equal(t, "", v["rdp_cert_fingerprint"])
	assert.NotEqual(t, "", v["ssh_host_keys"])
}
