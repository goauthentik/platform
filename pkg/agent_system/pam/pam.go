package pam

import (
	"context"

	"github.com/MicahParks/keyfunc/v3"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedPAMServer

	api *api.APIClient
	log *log.Entry
	kf  keyfunc.Keyfunc

	ctx    context.Context
	cancel context.CancelFunc

	cfg *config.Config
}

func NewServer() (component.Component, error) {
	dom := config.Get().Domains()[0]
	ac, err := dom.APIClient()
	if err != nil {
		return nil, err
	}
	srv := &Server{
		api: ac,
		log: log.WithField("logger", "sysd.pam_server"),
		cfg: config.Get(),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	k, err := keyfunc.NewDefaultCtx(srv.ctx, []string{ak.URLsForProfile(&storage.ConfigV1Profile{
		AuthentikURL: dom.AuthentikURL,
		AppSlug:      dom.AppSlug,
	}).JWKS})
	if err != nil {
		return nil, err
	}
	srv.kf = k
	return srv, nil
}

func (pam *Server) Start() {}

func (pam *Server) Stop() error {
	pam.cancel()
	return nil
}

func (pam *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterPAMServer(s, pam)
}
