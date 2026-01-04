package client

import (
	"context"
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type AgentClient struct {
	pb.AgentAuthClient
	pb.AgentCacheClient
	pb.AgentCtrlClient

	conn *grpc.ClientConn
}

func New(socketPath string) (*AgentClient, error) {
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
			grpc_sentry.UnaryClientInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			})),
		),
		grpc.WithChainStreamInterceptor(
			logging.StreamClientInterceptor(systemlog.InterceptorLogger(l)),
			grpc_sentry.StreamClientInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			})),
		),
	)
	if err != nil {
		return nil, err
	}
	return &AgentClient{
		pb.NewAgentAuthClient(conn),
		pb.NewAgentCacheClient(conn),
		pb.NewAgentCtrlClient(conn),
		conn,
	}, nil
}

func (c *AgentClient) Close() error {
	return c.conn.Close()
}
