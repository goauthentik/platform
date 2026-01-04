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
	assert.NoError(t, err)
	testcontainers.CleanupNetwork(t, net)

	req := testcontainers.ContainerRequest{
		Image:      "xghcr.io/goauthentik/platform-test:local",
		Entrypoint: []string{"/bin/bash", "-c", "sleep infinity"},
	}
	tc, err := testcontainers.GenericContainer(t.Context(), testcontainers.GenericContainerRequest{
		ContainerRequest: req,
		Started:          true,
	})
	assert.NoError(t, err)
	testcontainers.CleanupContainer(t, tc)

	assert.NoError(t, tc.Start(t.Context()))

	o := join(t, tc)
	t.Log(o)
}
