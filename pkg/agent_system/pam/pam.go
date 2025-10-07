package pam

import (
	"context"
	"errors"

	"github.com/MicahParks/keyfunc/v3"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	lconfig "goauthentik.io/cli/pkg/agent_local/config"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedPAMServer

	api *api.APIClient
	log *log.Entry
	kf  keyfunc.Keyfunc

	ctx context.Context

	cfg *config.Config
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log,
		cfg: config.Manager().Get(),
		ctx: ctx.Context,
	}
	return srv, nil
}

func (pam *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	dom := config.Manager().Get().Domains()[0]
	ac, err := dom.APIClient()
	if err != nil {
		return err
	}
	pam.api = ac
	k, err := keyfunc.NewDefaultCtx(pam.ctx, []string{ak.URLsForProfile(&lconfig.ConfigV1Profile{
		AuthentikURL: dom.AuthentikURL,
		AppSlug:      dom.AppSlug,
	}).JWKS})
	if err != nil {
		return err
	}
	pam.kf = k
	return nil
}

func (pam *Server) Stop() error {
	return nil
}

func (pam *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterPAMServer(s, pam)
}
