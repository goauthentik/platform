//go:build e2e

package e2e

import (
	"fmt"
	"io"
	"net/http"
	"os"
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

	// debug
	req, err := http.NewRequest("GET", LocalAuthentikURL()+"/api/v3/endpoints/devices/", nil)
	assert.NoError(t, err)
	req.Header.Set("Authorization", "Bearer "+os.Getenv("AK_TOKEN"))
	res, err := http.DefaultClient.Do(req)
	assert.NoError(t, err)
	b, err := io.ReadAll(res.Body)
	assert.NoError(t, err)
	t.Log(string(b))
	// end debug

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
