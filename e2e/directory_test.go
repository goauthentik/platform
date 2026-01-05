//go:build e2e

package e2e

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func Test_Directory_List(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))

	_ = join(t, tc)

	output := MustExec(t, tc, "getent passwd")
	t.Log(output)

	MustExec(t, tc, "journalctl -u ak-sysd")
}
