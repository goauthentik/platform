package directory

import (
	"context"
	"strconv"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	ctypes "goauthentik.io/platform/pkg/agent_system/ctrl/types"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/events"
	"google.golang.org/grpc"
)

const ID = "directory"

type Server struct {
	pb.UnimplementedSystemDirectoryServer

	log *log.Entry

	users  []*pb.User
	groups []*pb.Group

	ctx    component.Context
	cancel context.CancelFunc
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
		ctx: ctx,
	}
	return srv, nil
}

func (directory *Server) Start() error {
	directory.ctx.Bus().AddEventListener(ctypes.TopicCtrlDomainEnrolled, func(ev *events.Event) {
		if directory.cancel != nil {
			directory.cancel()
		}
		directory.startFetch()
	})
	directory.startFetch()
	return nil
}

func (directory *Server) Stop() error {
	return nil
}

func (directory *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterSystemDirectoryServer(s, directory)
}

func (directory *Server) GetUserUidNumber(cfg api.AgentConfig, user api.User) uint32 {
	uidNumber, ok := user.GetAttributes()["uidNumber"].(string)
	def := uint32(cfg.NssUidOffset + user.Pk)
	if ok {
		id, err := strconv.ParseUint(uidNumber, 10, 32)
		if err != nil {
			directory.log.WithField("user", user.Username).WithError(err).Warn("failed to get uid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (directory *Server) GetUserGidNumber(cfg api.AgentConfig, user api.User) uint32 {
	gidNumber, ok := user.GetAttributes()["gidNumber"].(string)
	def := directory.GetUserUidNumber(cfg, user)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			directory.log.WithField("user", user.Username).WithError(err).Warn("failed to get gid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (directory *Server) GetGroupGidNumber(cfg api.AgentConfig, group api.Group) uint32 {
	gidNumber, ok := group.GetAttributes()["gidNumber"].(string)
	def := uint32(cfg.NssGidOffset + group.NumPk)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			directory.log.WithField("group", group.Name).WithError(err).Warn("failed to get gid from group attributes")
			return def
		}
		return uint32(id)
	}
	return def
}
