package agentlocal

import (
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent_local/grpc_creds"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

func (a *Agent) startGRPC() {
	l := a.log.WithField("logger", "agent.grpc")
	lis, err := net.Listen("unix", a.socketPath)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	a.grpc = grpc.NewServer(
		grpc.Creds(grpc_creds.NewTransportCredentials()),
		grpc.ChainUnaryInterceptor(
			logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l)),
		),
		grpc.ChainStreamInterceptor(
			logging.StreamServerInterceptor(systemlog.InterceptorLogger(l)),
		),
	)
	pb.RegisterAgentAuthServer(a.grpc, a)
	pb.RegisterAgentCacheServer(a.grpc, a)
	pb.RegisterAgentConfigServer(a.grpc, a)
	log.WithField("socket", a.socketPath).Info("Starting GRPC server")
	if err := a.grpc.Serve(lis); err != nil {
		log.WithError(err).Fatal("Failed to serve")
	}
}
