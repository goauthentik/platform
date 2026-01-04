//go:build integration

package device

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/sysd/component"
	"goauthentik.io/platform/pkg/sysd/config"
)

func TestCheckin(t *testing.T) {
	dc := config.IntegrationDomain(t)
	ds, err := NewServer(component.TestContext(t, dc))
	assert.NoError(t, err)

	dds := ds.(*Server)
	assert.NoError(t, dds.checkIn(t.Context(), dc))
}
