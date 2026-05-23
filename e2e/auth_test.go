//go:build e2e

package e2e

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func Test_Auth_Legacy(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))
	JoinDomain(t, tc)
	AgentSetup(t, tc)

	MustExec(t, tc, "sed -i 's/KbdInteractiveAuthentication no/KbdInteractiveAuthentication yes/g' /etc/ssh/sshd_config")
	MustExec(t, tc, "systemctl restart ssh")

	cmdTest(t, tc, []cmdTestCase{
		{
			name:    "ssh_env",
			cmd:     "ak ssh -i akadmin@$(hostname) env",
			expects: []string{"AUTHENTIK_CLI_SOCKET", "SSH_CONNECTION"},
		},
		{
			name:    "ssh_ak_whoami",
			cmd:     "ak ssh -i akadmin@$(hostname) ak whoami",
			expects: []string{"akadmin"},
		},
	})
}
