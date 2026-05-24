//go:build e2e

package e2e

import (
	"fmt"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func Test_Auth_IdentityAgent(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))
	JoinDomain(t, tc)
	AgentSetup(t, tc)

	sshOpts := []string{
		"-o StrictHostKeyChecking=no",
		"-o IdentityAgent=~/.local/share/authentik/agent-ssh.sock",
		"-o ForwardAgent=yes",
	}
	cmdTest(t, tc, []cmdTestCase{
		{
			name:    "ssh_env",
			cmd:     fmt.Sprintf("ssh %s akadmin@$(hostname) env", strings.Join(sshOpts, " ")),
			expects: []string{"SSH_CONNECTION"},
		},
		{
			name:    "ssh_ak_whoami",
			cmd:     fmt.Sprintf("ssh %s akadmin@$(hostname) ak whoami", strings.Join(sshOpts, " ")),
			expects: []string{"akadmin"},
		},
	})
}
