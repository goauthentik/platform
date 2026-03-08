package agentlocal

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/socket"
	"goauthentik.io/platform/pkg/shared"
	"google.golang.org/grpc"
)

func (a *Agent) startGRPC() {
	l := a.log.WithField("logger", "agent.grpc")
	lis, err := socket.Listen(types.GetAgentSocketPath(), socket.SocketOwner)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	a.lis = lis
	a.grpc = grpc.NewServer(
		shared.CommonGRPCServerOpts(l, grpc.Creds(grpc_creds.NewTransportCredentials(l.WithField("logger", "agent.grpc.auth"))))...,
	)
	pb.RegisterAgentAuthServer(a.grpc, a)
	pb.RegisterAgentCacheServer(a.grpc, a)
	pb.RegisterAgentCtrlServer(a.grpc, a)
	pb.RegisterPingServer(a.grpc, a)
	a.log.WithField("socket", lis.Path().ForCurrent()).Info("Starting GRPC server")
	if err := a.grpc.Serve(lis); err != nil {
		a.log.WithError(err).Fatal("Failed to serve")
	}
}
