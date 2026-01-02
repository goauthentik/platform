package client

import (
	"context"
	"net"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	grpc_sentry "github.com/johnbellone/grpc-middleware-sentry"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type SysdClient struct {
	pb.SessionManagerClient
	pb.AgentPlatformClient
	pb.PingClient
	pb.SystemCtrlClient

	conn *grpc.ClientConn
}

func NewDefault() (*SysdClient, error) {
	return New(types.GetSysdSocketPath(types.SocketIDDefault))
}

func NewCtrl() (*SysdClient, error) {
	return New(types.GetSysdSocketPath(types.SocketIDCtrl))
}

func New(path pstr.PlatformString) (*SysdClient, error) {
	l := log.WithField("logger", "cli.system_grpc")
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(path)
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
	return &SysdClient{
		pb.NewSessionManagerClient(conn),
		pb.NewAgentPlatformClient(conn),
		pb.NewPingClient(conn),
		pb.NewSystemCtrlClient(conn),
		conn,
	}, nil
}

func (c *SysdClient) Close() error {
	return c.conn.Close()
}
