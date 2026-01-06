//go:build e2e

package e2e

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func Test_Auth(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)
	// tcip, err := tc.Host(t.Context())
	// assert.NoError(t, err)
	// tcport, err := tc.MappedPort(t.Context(), "22")
	// assert.NoError(t, err)
	// port, _ := strconv.Atoi(tcport.Port())

	assert.NoError(t, tc.Start(t.Context()))
	JoinDomain(t, tc)
	AgentSetup(t)

	MustExec(t, tc, "ak ssh -i akadmin@localhost w")

	// agentClient, err := client.New(types.GetAgentSocketPath().ForCurrent())
	// assert.NoError(t, err)

	// c, err := ssh.New(tcip, port, "akadmin")
	// assert.NoError(t, err)
	// c.Insecure = true
	// c.AgentClient = agentClient
	// c.AgentProfile = "ak"
	// c.Command = "w"

	// assert.NoError(t, c.Connect())
}
