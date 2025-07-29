package cli

import (
	"context"
	"net"
	"os"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func sessionClient() (pb.SessionManagerClient, error) {
	l := log.WithField("logger", "cli.system_grpc")
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return net.Dial("unix", "/var/run/authentik-session-manager.sock")
		}),
		grpc.WithUnaryInterceptor(logging.UnaryClientInterceptor(systemlog.InterceptorLogger(l))),
		grpc.WithStreamInterceptor(logging.StreamClientInterceptor(systemlog.InterceptorLogger(l))),
	)
	if err != nil {
		return nil, err
	}
	return pb.NewSessionManagerClient(conn), nil
}

var sessionCmd = &cobra.Command{
	Use:   "session",
	Short: "Commands for interacting with authentik sessions.",
}

func init() {
	if _, err := os.Stat("/var/run/authentik-session-manager.sock"); err == nil {
		rootCmd.AddCommand(sessionCmd)
	}
}
