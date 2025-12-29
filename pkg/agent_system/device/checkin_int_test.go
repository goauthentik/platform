//go:build integration

package device

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
)

func TestCheckin(t *testing.T) {
	t.Skip()
	ds, err := NewServer(component.TestContext(t))
	assert.NoError(t, err)

	dds := ds.(*Server)
	assert.NoError(t, dds.checkIn(t.Context(), config.IntegrationDomain(t)))
}
