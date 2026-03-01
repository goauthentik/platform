package ctrl

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/storage/state"
	"google.golang.org/grpc"
)

const ID = "ctrl"

type Server struct {
	pb.UnimplementedSystemCtrlServer

	log *log.Entry
	ctx component.Context
	rst *state.State
}

func NewServer(ctx component.Context) (component.Component, error) {
	return &Server{
		log: ctx.Log(),
		ctx: ctx,
		rst: config.State(),
	}, nil
}

func (ctrl *Server) Start() error {
	return nil
}

func (ctrl *Server) Stop() error {
	return nil
}

func (ctrl *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDCtrl {
		return
	}
	pb.RegisterSystemCtrlServer(s, ctrl)
}
