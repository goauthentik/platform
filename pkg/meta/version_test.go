package meta

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func Test_VersionBranch(t *testing.T) {
	Version = "1.2.3"
	BuildHash = "foobarbaz"
	Tag = ""
	assert.Equal(t, "1.2.3-foobarba", FullVersion())
	assert.Equal(t, "https://github.com/goauthentik/platform/commit/foobarbaz", BuildURL())
}

func Test_Tag(t *testing.T) {
	Version = "1.2.3"
	BuildHash = "foobarbaz"
	Tag = "v1.2.3"
	assert.Equal(t, "1.2.3", FullVersion())
}

func Test_VersionBranch_Short(t *testing.T) {
	Version = "1.2.3"
	BuildHash = "foobarb"
	Tag = ""
	assert.Equal(t, "1.2.3-foobarb", FullVersion())
}
