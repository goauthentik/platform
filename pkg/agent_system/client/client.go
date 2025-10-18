package client

import (
	"context"
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type SysdClient struct {
	pb.SessionManagerClient
	pb.AgentPlatformClient

	conn *grpc.ClientConn
}

func New() (*SysdClient, error) {
	l := log.WithField("logger", "cli.system_grpc")
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(types.GetSysdSocketPath())
		}),
		grpc.WithUnaryInterceptor(logging.UnaryClientInterceptor(systemlog.InterceptorLogger(l))),
		grpc.WithStreamInterceptor(logging.StreamClientInterceptor(systemlog.InterceptorLogger(l))),
	)
	if err != nil {
		return nil, err
	}
	return &SysdClient{
		pb.NewSessionManagerClient(conn),
		pb.NewAgentPlatformClient(conn),
		conn,
	}, nil
}

func (c *SysdClient) Close() error {
	return c.conn.Close()
}
