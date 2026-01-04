//go:build e2e

package e2e

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func Test_Auth(t *testing.T) {
	t.Skip()
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc, err := testcontainers.GenericContainer(t.Context(), endpointTestContainer(t))
	defer testcontainers.CleanupContainer(t, tc)
	assert.NoError(t, err)

	assert.NoError(t, tc.Start(t.Context()))

	o := join(t, tc)
	t.Log(o)
}
