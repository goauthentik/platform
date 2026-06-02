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

// Test_Auth_LocalOnlyUser verifies that a user unknown to authentik is PAM_IGNOREd
// by the authentik module, allowing the rest of the PAM stack to handle account
// management so the user can still log in via local credentials.
func Test_Auth_LocalOnlyUser(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))
	JoinDomain(t, tc)

	// Create a local user that is not registered in authentik.
	MustExec(t, tc, "useradd -m localonly")
	MustExec(t, tc, "ssh-keygen -t ed25519 -f /tmp/localonly_key -N '' -q")
	MustExec(t, tc, "install -d -m 700 -o localonly -g localonly /home/localonly/.ssh")
	MustExec(t, tc, "cp /tmp/localonly_key.pub /home/localonly/.ssh/authorized_keys")
	MustExec(t, tc, "chown localonly: /home/localonly/.ssh/authorized_keys && chmod 600 /home/localonly/.ssh/authorized_keys")

	sshOpts := "-i /tmp/localonly_key -o StrictHostKeyChecking=no -o BatchMode=yes"

	cmdTest(t, tc, []cmdTestCase{
		{
			// The authentik PAM module returns PAM_IGNORE for this user;
			// pam_unix then handles account management and the login succeeds.
			name:    "local_only_user_can_ssh",
			cmd:     "ssh " + sshOpts + " localonly@localhost whoami",
			expects: []string{"localonly"},
		},
		{
			// Sanity-check: the local user is NOT visible through the authentik
			// NSS module (getent should not return an authentik entry for them).
			name:    "local_only_user_not_in_authentik_directory",
			cmd:     "getent passwd localonly",
			expects: []string{"localonly:/home/localonly"},
		},
	})
}
