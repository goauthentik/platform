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

	assert.NoError(t, tc.Start(t.Context()))
	JoinDomain(t, tc)
	AgentSetup(t, tc)

	for _, testCase := range []cmdTestCase{
		{
			cmd:     "ak ssh -i akadmin@$(hostname) env",
			expects: []string{"AUTHENTIK_CLI_SOCKET", "SSH_CONNECTION"},
		},
		{
			cmd:     "ak ssh -i akadmin@$(hostname) ak whoami",
			expects: []string{"akadmin"},
		},
	} {
		t.Run(testCase.cmd, func(t *testing.T) {
			output := MustExec(t, tc, testCase.cmd)
			for _, expect := range testCase.expects {
				assert.Contains(t, output, expect)
			}
		})
	}
}
