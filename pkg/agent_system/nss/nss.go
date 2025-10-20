package nss

import (
	"context"
	"errors"
	"strconv"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedNSSServer

	api *api.APIClient
	log *log.Entry

	users  []*pb.User
	groups []*pb.Group

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

func (nss *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	ac, err := config.Manager().Get().Domains()[0].APIClient()
	if err != nil {
		return err
	}
	nss.api = ac
	nss.startFetch()
	return nil
}

func (nss *Server) Stop() error {
	return nil
}

func (nss *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterNSSServer(s, nss)
}

func (nss *Server) GetUserUidNumber(user api.User) uint32 {
	uidNumber, ok := user.GetAttributes()["uidNumber"].(string)
	def := uint32(nss.cfg.NSS.UIDOffset + user.Pk)
	if ok {
		id, err := strconv.ParseUint(uidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("user", user.Username).WithError(err).Warn("failed to get uid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (nss *Server) GetUserGidNumber(user api.User) uint32 {
	gidNumber, ok := user.GetAttributes()["gidNumber"].(string)
	def := nss.GetUserUidNumber(user)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("user", user.Username).WithError(err).Warn("failed to get gid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (nss *Server) GetGroupGidNumber(group api.Group) uint32 {
	gidNumber, ok := group.GetAttributes()["gidNumber"].(string)
	def := uint32(nss.cfg.NSS.GIDOffset + group.NumPk)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("group", group.Name).WithError(err).Warn("failed to get gid from group attributes")
			return def
		}
		return uint32(id)
	}
	return def
}
