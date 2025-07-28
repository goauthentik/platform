package client

import (
	"context"
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Client struct {
	pb.AgentAuthClient
	pb.AgentCacheClient
}

func New(socketPath string) (*Client, error) {
	l := log.WithField("logger", "cli.grpc")
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return net.Dial("unix", socketPath)
		}),
		grpc.WithUnaryInterceptor(logging.UnaryClientInterceptor(systemlog.InterceptorLogger(l))),
		grpc.WithStreamInterceptor(logging.StreamClientInterceptor(systemlog.InterceptorLogger(l))),
	)
	if err != nil {
		return nil, err
	}
	return &Client{
		pb.NewAgentAuthClient(conn),
		pb.NewAgentCacheClient(conn),
	}, nil
}
