package client

import (
	"context"
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/pb"
	systemlog "goauthentik.io/cli/pkg/platform/log"
	"goauthentik.io/cli/pkg/platform/pstr"
	"goauthentik.io/cli/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Client struct {
	pb.AgentAuthClient
	pb.AgentCacheClient
	pb.AgentConfigClient
}

func New(socketPath string) (*Client, error) {
	l := log.WithField("logger", "cli.grpc")
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(pstr.PlatformString{
				Fallback: socketPath,
			})
		}),
		grpc.WithChainUnaryInterceptor(
			logging.UnaryClientInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.UnaryClientInterceptor(),
		),
		grpc.WithChainStreamInterceptor(
			logging.StreamClientInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.StreamClientInterceptor(),
		),
	)
	if err != nil {
		return nil, err
	}
	return &Client{
		pb.NewAgentAuthClient(conn),
		pb.NewAgentCacheClient(conn),
		pb.NewAgentConfigClient(conn),
	}, nil
}
