package agentlocal

import (
	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/socket"
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
		grpc.Creds(grpc_creds.NewTransportCredentials()),
		grpc.ChainUnaryInterceptor(
			logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.UnaryServerInterceptor(),
		),
		grpc.ChainStreamInterceptor(
			logging.StreamServerInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.StreamServerInterceptor(),
		),
	)
	pb.RegisterAgentAuthServer(a.grpc, a)
	pb.RegisterAgentCacheServer(a.grpc, a)
	pb.RegisterAgentConfigServer(a.grpc, a)
	a.log.WithField("socket", lis.Path().ForCurrent()).Info("Starting GRPC server")
	if err := a.grpc.Serve(lis); err != nil {
		a.log.WithError(err).Fatal("Failed to serve")
	}
}
