package agentsystem

import (
	"net"
	"os"
	"os/signal"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type SessionManager struct {
	pb.UnimplementedSessionManagerServer
	sessions map[string]*Session
	monitor  *SessionMonitor
	lis      net.Listener
	srv      *grpc.Server
	log      *log.Entry
}

func New() *SessionManager {
	_ = os.Remove(config.Get().Socket)

	lis, err := net.Listen("unix", config.Get().Socket)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	_ = os.Chmod(config.Get().Socket, 0666)

	l := log.WithField("logger", "agent_sys.sm")
	sm := &SessionManager{
		sessions: make(map[string]*Session),
		monitor:  NewSessionMonitor(),
		lis:      lis,
		srv: grpc.NewServer(
			grpc.ChainUnaryInterceptor(logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l))),
			grpc.ChainStreamInterceptor(logging.StreamServerInterceptor(systemlog.InterceptorLogger(l))),
		),
		log: l,
	}
	pb.RegisterSessionManagerServer(sm.srv, sm)
	return sm
}

func (sm *SessionManager) Start() {
	// Start session monitor
	go sm.monitor.Start(sm.sessions)

	// Handle graceful shutdown
	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		sm.log.Info("Shutting down...")
		sm.srv.GracefulStop()
		_ = os.Remove(config.Get().Socket)
	}()

	sm.log.Infof("Session manager listening on socket: %s", config.Get().Socket)
	if err := sm.srv.Serve(sm.lis); err != nil {
		sm.log.Fatalf("Failed to serve: %v", err)
	}
}
