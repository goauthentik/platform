package agentlocal

import (
	"context"

	"goauthentik.io/platform/pkg/agent_local/types"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
)

func (a *Agent) startSSH() {
	l := a.log.WithField("logger", "agent.ssh")
	vgrpc := &sshagent.MethodCaller{}
	a.setupGRPCServer(vgrpc)
	ag, err := sshagent.New(l, a.tr, context.Background(), vgrpc)
	if err != nil {
		a.log.WithError(err).Warning("failed to init SSH agent")
		return
	}
	ag.Profile = "default"
	a.ssh = ag
	err = ag.Listen(types.GetAgentSocketPath(types.SocketIDSSH))
	if err != nil {
		a.log.WithError(err).Warn("Failed to serve")
	}
}
