package agentsystem

import (
	"context"
	"fmt"
	"net"
	"net/url"
	"os"
	"os/signal"
	"syscall"

	"github.com/grpc-ecosystem/go-grpc-middleware/v2/interceptors/logging"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/agent_system/nss"
	"goauthentik.io/cli/pkg/agent_system/session"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

var agentCmd = &cobra.Command{
	Use:          "agent",
	Short:        "Run the authentik system agent",
	SilenceUsage: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		err := agentPrecheck()
		if err != nil {
			return err
		}
		if _, err := os.Stat(config.Get().RuntimeDir()); err != nil {
			return errors.Wrap(err, "failed to check runtime directory")
		}
		return nil
	},
	Run: func(cmd *cobra.Command, args []string) {
		log.SetLevel(log.DebugLevel)
		err := systemlog.Setup("ak-sys-agent")
		if err != nil {
			panic(err)
		}
		New().Start()
	},
}

func init() {
	rootCmd.AddCommand(agentCmd)
}

type SystemAgent struct {
	nss     *nss.Server
	monitor *session.Monitor
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

	ac := api.NewAPIClient(apiConfig)

	m, _, err := ac.CoreApi.CoreUsersMeRetrieve(context.Background()).Execute()
	if err != nil {
		panic(err)
	}
	l.WithField("as", m.User.Username).Debug("Connected to authentik")

	sm := &SystemAgent{
		monitor: session.NewMonitor(),
		srv: grpc.NewServer(
			grpc.ChainUnaryInterceptor(logging.UnaryServerInterceptor(systemlog.InterceptorLogger(l))),
			grpc.ChainStreamInterceptor(logging.StreamServerInterceptor(systemlog.InterceptorLogger(l))),
		),
		log: l,
		api: ac,
		nss: nss.NewServer(ac),
	}

	pb.RegisterSessionManagerServer(sm.srv, sm.monitor)
	pb.RegisterNSSServer(sm.srv, sm.nss)
	return sm
}

func (sm *SystemAgent) Start() {
	go sm.monitor.Start()

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		sm.log.Info("Shutting down...")
		sm.srv.GracefulStop()
		_ = os.Remove(config.Get().Socket)
	}()

	_ = os.Remove(config.Get().Socket)
	lis, err := net.Listen("unix", config.Get().Socket)
	if err != nil {
		log.WithError(err).Fatal("Failed to listen")
	}
	_ = os.Chmod(config.Get().Socket, 0666)

	sm.log.WithField("path", config.Get().Socket).Info("System agent listening on socket")
	if err := sm.srv.Serve(lis); err != nil {
		sm.log.WithError(err).Fatal("Failed to serve")
	}
}
