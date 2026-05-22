package agentlocal

import (
	"goauthentik.io/platform/pkg/agent_local/types"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
)

func (a *Agent) startSSH() {
	l := a.log.WithField("logger", "agent.ssh")
	ag, err := sshagent.New(l, a.tr)
	if err != nil {
		a.log.WithError(err).Warning("failed to init SSH agent")
		return
	}
	err = ag.Listen(types.GetAgentSocketPath(types.SocketIDSSH))
	if err != nil {
		a.log.WithError(err).Warn("Failed to serve")
	}
}
