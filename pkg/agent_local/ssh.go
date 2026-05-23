package agentlocal

import (
	"context"

	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/shared/grpc"
	sshagent "goauthentik.io/platform/pkg/ssh_agent"
)

func (a *Agent) startSSH() {
	l := a.log.WithField("logger", "agent.ssh")
	vgrpc := &grpc.MethodCaller{}
	a.setupGRPCServer(vgrpc)
	ag, err := sshagent.New(l, a.tr, context.Background(), vgrpc)
	if err != nil {
		a.log.WithError(err).Warning("failed to init SSH agent")
		return
	}
	ag.Profile = "default"
	a.ssh = ag
	path := types.GetAgentSocketPath(types.SocketIDSSH)
	a.log.WithField("socket", path).Info("Starting SSH Agent")
	err = ag.Listen(path)
	if err != nil {
		a.log.WithError(err).Warn("Failed to serve")
	}
}
