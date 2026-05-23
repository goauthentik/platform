package agentlocal

import (
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/shared"
	"google.golang.org/grpc"
)

func (a *Agent) setupGRPCServer(grpc grpc.ServiceRegistrar) {
	pb.RegisterAgentAuthServer(grpc, a)
	pb.RegisterAgentCacheServer(grpc, a)
	pb.RegisterAgentCtrlServer(grpc, a)
	pb.RegisterPingServer(grpc, a)
}

func (a *Agent) startGRPC() {
	l := a.log.WithField("logger", "agent.grpc")
	lis, err := socket.Listen(types.GetAgentSocketPath(types.SocketIDDefault), socket.SocketOwner)
	if err != nil {
		a.log.WithError(err).Fatal("Failed to listen")
	}
	grpc := grpc.NewServer(
		shared.CommonGRPCServerOpts(l, grpc.Creds(grpc_creds.NewTransportCredentials(l.WithField("logger", "agent.grpc.auth"))))...,
	)
	a.setupGRPCServer(grpc)
	a.grpc = grpc
	a.lis = lis
	a.log.WithField("socket", lis.Path().ForCurrent()).Info("Starting GRPC server")
	if err := a.grpc.Serve(lis); err != nil {
		a.log.WithError(err).Fatal("Failed to serve")
	}
}
