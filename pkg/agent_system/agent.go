package agentsystem

import (
	"fmt"
	"net"
	"net/url"
	"os"
	"os/signal"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type SystemAgent struct {
	pb.UnimplementedSessionManagerServer
	pb.UnimplementedNSSServer

	monitor *SessionMonitor
	srv     *grpc.Server
	log     *log.Entry
	api     *api.APIClient
}

func New() *SystemAgent {
	l := log.WithField("logger", "agent_sys.sm")

	u, err := url.Parse(config.Get().AuthentikURL)
	if err != nil {
		panic(err)
	}
	apiConfig := api.NewConfiguration()
	apiConfig.Host = u.Host
	apiConfig.Scheme = u.Scheme
	apiConfig.Servers = api.ServerConfigurations{
		{
			URL: fmt.Sprintf("%sapi/v3", u.Path),
		},
	}
	apiConfig.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", config.Get().Token))

	sm := &SystemAgent{
		monitor: NewSessionMonitor(),
		srv: grpc.NewServer(
			grpc.ChainUnaryInterceptor(logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l))),
			grpc.ChainStreamInterceptor(logging.StreamServerInterceptor(systemlog.InterceptorLogger(l))),
		),
		log: l,
		api: api.NewAPIClient(apiConfig),
	}
	pb.RegisterSessionManagerServer(sm.srv, sm)
	pb.RegisterNSSServer(sm.srv, sm)
	return sm
}

func (sa *SystemAgent) Start() {
	go sa.monitor.Start()

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		sa.log.Info("Shutting down...")
		sa.srv.GracefulStop()
		_ = os.Remove(config.Get().Socket)
	}()

	_ = os.Remove(config.Get().Socket)
	lis, err := net.Listen("unix", config.Get().Socket)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}
	_ = os.Chmod(config.Get().Socket, 0666)

	sa.log.Infof("Session manager listening on socket: %s", config.Get().Socket)
	if err := sa.srv.Serve(lis); err != nil {
		sa.log.Fatalf("Failed to serve: %v", err)
	}
}
