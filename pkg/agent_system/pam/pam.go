package pam

import (
	"context"

	"github.com/MicahParks/keyfunc/v3"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
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

func NewServer(api *api.APIClient) (*Server, error) {
	srv := &Server{
		api: api,
		log: log.WithField("logger", "sysd.pam_server"),
		cfg: config.Get(),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	k, err := keyfunc.NewDefaultCtx(srv.ctx, []string{ak.URLsForProfile(&storage.ConfigV1Profile{
		AuthentikURL: srv.cfg.AK.AuthentikURL,
		AppSlug:      srv.cfg.AK.AppSlug,
	}).JWKS})
	if err != nil {
		return nil, err
	}
	srv.kf = k
	return srv, nil
}

func (pam *Server) Stop() {
	pam.cancel()
}
