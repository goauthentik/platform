package directory

import (
	"context"
	"errors"
	"strconv"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

const ID = "directory"

type Server struct {
	pb.UnimplementedSystemDirectoryServer

	api *api.APIClient
	log *log.Entry

	users  []*pb.User
	groups []*pb.Group

	ctx context.Context

	cfg *api.AgentConfig
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
		ctx: ctx.Context(),
	}
	return srv, nil
}

func (directory *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	dom := config.Manager().Get().Domains()[0]
	ac, err := dom.APIClient()
	if err != nil {
		return err
	}
	directory.api = ac
	directory.cfg = dom.Config()
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

func (directory *Server) GetUserUidNumber(user api.User) uint32 {
	uidNumber, ok := user.GetAttributes()["uidNumber"].(string)
	def := uint32(directory.cfg.NssUidOffset + user.Pk)
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

func (directory *Server) GetUserGidNumber(user api.User) uint32 {
	gidNumber, ok := user.GetAttributes()["gidNumber"].(string)
	def := directory.GetUserUidNumber(user)
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

func (directory *Server) GetGroupGidNumber(group api.Group) uint32 {
	gidNumber, ok := group.GetAttributes()["gidNumber"].(string)
	def := uint32(directory.cfg.NssGidOffset + group.NumPk)
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
