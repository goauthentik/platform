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
	pb.PingClient

	conn *grpc.ClientConn
	log  *log.Entry
}

type opt func(ac *AgentClient) (grpc.UnaryClientInterceptor, grpc.StreamClientInterceptor)

func sentryOpt() opt {
	return func(ac *AgentClient) (grpc.UnaryClientInterceptor, grpc.StreamClientInterceptor) {
		return grpc_sentry.UnaryClientInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			})), grpc_sentry.StreamClientInterceptor(grpc_sentry.WithReportOn(func(error) bool {
				return false
			}))
	}
}

func WithLogging() opt {
	return func(ac *AgentClient) (grpc.UnaryClientInterceptor, grpc.StreamClientInterceptor) {
		return logging.UnaryClientInterceptor(systemlog.InterceptorLogger(ac.log)),
			logging.StreamClientInterceptor(systemlog.InterceptorLogger(ac.log))
	}
}

func New(socketPath string, opts ...opt) (*AgentClient, error) {
	l := log.WithField("logger", "cli.grpc")
	ag := &AgentClient{
		log: l,
	}
	clientInterceptors := []grpc.UnaryClientInterceptor{}
	streamInterceptors := []grpc.StreamClientInterceptor{}
	opts = append(opts, sentryOpt())
	for _, opt := range opts {
		c, s := opt(ag)
		clientInterceptors = append(clientInterceptors, c)
		streamInterceptors = append(streamInterceptors, s)
	}
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(pstr.PlatformString{
				Fallback: socketPath,
			})
		}),
		grpc.WithChainUnaryInterceptor(clientInterceptors...),
		grpc.WithChainStreamInterceptor(streamInterceptors...),
	)
	if err != nil {
		return nil, err
	}
	ag.conn = conn
	ag.AgentAuthClient = pb.NewAgentAuthClient(conn)
	ag.AgentCacheClient = pb.NewAgentCacheClient(conn)
	ag.AgentCtrlClient = pb.NewAgentCtrlClient(conn)
	ag.PingClient = pb.NewPingClient(conn)
	return ag, nil
}

func (c *AgentClient) Close() error {
	return c.conn.Close()
}
