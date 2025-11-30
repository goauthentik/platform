package ctrl

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "ctrl"

type Server struct {
	pb.UnimplementedSystemCtrlServer

	log *log.Entry
	ctx component.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	return &Server{
		log: ctx.Log(),
		ctx: ctx,
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
